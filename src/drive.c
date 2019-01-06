/*
 * Copyright 2018: Alistair Francis <alistair@alistair23.me>
 *
 * See the LICENSE file for license information.
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
 * THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
 * THE SOFTWARE.
 */

#include <Python.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <math.h>
#include <gps.h>
#include "track.h"
#include "common.h"
#include "drive.h"
#include "obdii.h"

typedef struct drive_args {
	OsmGpsMap *map;
	struct gps_data_t gps_data;
	track *cur_track;
} drive_args;

gboolean time_drive_loop(gpointer user_data)
{
	drive_loop_data *drive_data = user_data;
	gtk_user_data *data = drive_data->data;
	struct timespec *start_time = drive_data->start_time;
	struct timespec cur_time, diff_time;
	gchar *clock_time;
	const char *format = TIMER_FORMAT;
	char *markup;

	g_assert(g_main_context_get_thread_default() == g_main_context_default() ||
			g_main_context_get_thread_default() == NULL);

	if (!data || data->finished_drive) {
		return false;
	}

	clock_gettime(CLOCK_MONOTONIC_RAW, &cur_time);
	diff_time = timeval_subtract(&cur_time, start_time);
	clock_time = g_strdup_printf("%02ld:%02ld:%02ld",
								diff_time.tv_sec / 60,
								diff_time.tv_sec % 60,
								diff_time.tv_nsec / (1000 * 1000 * 10));
	markup = g_markup_printf_escaped(format, clock_time);
	gtk_label_set_markup(GTK_LABEL(data->ddisp_widgets[TIMER]), markup);
	g_free(clock_time);
	g_free(markup);

	return true;
}

static gboolean map_drive_update(gpointer drive_data)
{
	drive_args *args = drive_data;
	OsmGpsMap *map = args->map;
	struct gps_data_t gps_data = args->gps_data;
	track *cur_track = args->cur_track;

	g_assert(g_main_context_get_thread_default() == g_main_context_default() ||
			g_main_context_get_thread_default() == NULL);

	osm_gps_map_gps_add(map,
						gps_data.fix.latitude,
						gps_data.fix.longitude,
						gps_data.fix.track);

	if (!cur_track) {
		/* We don't have a map loaded */
		osm_gps_map_set_center_and_zoom(map,
					gps_data.fix.latitude,
					gps_data.fix.longitude,
					MAP_ZOOM_LEVEL);
	}

	return false;
}

static void map_drive_update_notify_free(gpointer data)
{
	g_free(data);
}


gboolean map_drive_loop(gpointer user_data)
{
	drive_loop_data *drive_data = user_data;
	gtk_user_data *data = drive_data->data;
	struct gps_data_t gps_data = drive_data->gps_data;
	OsmGpsMap *map = drive_data->map;
	track *cur_track = drive_data->cur_track;
	drive_args *args = g_new0(drive_args, 1);
	int ret;

	if (!data || data->finished_drive) {
		g_main_loop_quit(data->drive_loop);
		return false;
	}

	if (gps_waiting(&gps_data, 500)) {
		ret = gps_read(&gps_data, NULL, 0);

		if (ret < 0) {
			fprintf(stderr, "gps_read error: %d\n", ret);
			exit(1);
		}

		if (!isnan(gps_data.fix.latitude) &&
			!isnan(gps_data.fix.longitude)) {
			args->map = map;
			args->gps_data = gps_data;
			args->cur_track = cur_track;

			g_main_context_invoke_full(g_main_context_default(),
										G_PRIORITY_LOW,
										map_drive_update, args,
										map_drive_update_notify_free);

			if (cur_track &&
				equal(gps_data.fix.latitude, cur_track->end.lat, 0.0005) &&
				equal(gps_data.fix.longitude, cur_track->end.lon, 0.0005)) {
				g_main_loop_quit(data->drive_loop);
				data->finished_drive = true;
				return false;
			}
		}
	}

	return true;
}

gpointer prepare_to_drive(gpointer user_data)
{
	gtk_user_data *data = user_data;
	cmd_args args = *data->args;
	struct gps_data_t gps_data;
	struct timespec cur_time, diff_time;
	track *cur_track = NULL;
	struct timespec *start_time;
	OsmGpsMap *map = OSM_GPS_MAP(data->drive_map);
	int ret;
	GMainContext *worker_context;
	GSource *source_1, *source_2;
	gchar *clock_time;
	const char *format = TIMER_FORMAT;
	char *markup;

	worker_context = g_main_context_new();
	g_main_context_push_thread_default(worker_context);

	gps_data = connect_to_gpsd(args);
	gps_stream(&gps_data, WATCH_ENABLE | WATCH_JSON, NULL);

	while (data->load_page) {
		if (data->drive_track_filepath && data->drive_track_updated) {
			osm_gps_map_track_remove_all(map);

				gtk_button_set_label(GTK_BUTTON(data->drive_file_download_button),
						"Download this map");

			cur_track = load_track(data->drive_track_filepath, false);
			if (cur_track->osm_track) {
				data->loaded_track = cur_track;

				osm_gps_map_set_center_and_zoom(map, cur_track->start.lat, cur_track->start.lon, MAP_ZOOM_LEVEL);
				osm_gps_map_track_add(map, cur_track->osm_track);

				data->drive_track_updated = false;
			}
		}
	}

	/* Update this */
	map = OSM_GPS_MAP(data->drive_map);

	if (cur_track) {
		start_time = &cur_track->start.time;
	} else {
		start_time = g_new0(struct timespec, 1);
		clock_gettime(CLOCK_MONOTONIC_RAW, start_time);
	}

	g_object_ref(data->drive_container);

	/* Poll until we hit the start line */
	while (cur_track && !data->finished_drive) {
		if (gps_waiting(&gps_data, 500)) {
			ret = gps_read(&gps_data, NULL, 0);

			if (ret < 0) {
				fprintf(stderr, "gps_read error: %d\n", ret);
				exit(1);
			}

			if (!isnan(gps_data.fix.latitude) &&
				!isnan(gps_data.fix.longitude)) {
				osm_gps_map_gps_add(map,
									gps_data.fix.latitude,
									gps_data.fix.longitude,
									gps_data.fix.track);

				if (cur_track &&
					equal(gps_data.fix.latitude, cur_track->start.lat, 0.0005) &&
					equal(gps_data.fix.longitude, cur_track->start.lon, 0.0005)) {
					clock_gettime(CLOCK_MONOTONIC_RAW, start_time);
					break;
				}
			}
		}
	}

	fprintf(stderr, "Starting the drive\n");

	drive_loop_data *drive_data = g_new0(drive_loop_data, 1);
	drive_data->data = data;
	drive_data->gps_data = gps_data;
	drive_data->start_time = start_time;
	drive_data->map = map;
	drive_data->cur_track = cur_track;

	data->drive_loop = g_main_loop_new(worker_context, false);

	source_1 = g_timeout_source_new(10);
	g_source_set_callback(source_1, time_drive_loop, drive_data, NULL);
	/* Run in main loop */
	g_source_attach(source_1, g_main_context_default());

	source_2 = g_timeout_source_new(500);
	g_source_set_callback(source_2, map_drive_loop, drive_data, NULL);
	/* Run in this thread loop */
	g_source_attach(source_2, worker_context);

	g_main_context_unref(worker_context);
	g_source_unref(source_1);
	g_source_unref(source_2);

	g_main_loop_run(data->drive_loop);
	g_main_loop_unref(data->drive_loop);

	g_source_destroy(source_1);

	g_free(drive_data);

	clock_gettime(CLOCK_MONOTONIC_RAW, &cur_time);
	diff_time = timeval_subtract(&cur_time, start_time);
	clock_time = g_strdup_printf("%02ld:%02ld:%02ld",
								diff_time.tv_sec / 60,
								diff_time.tv_sec % 60,
								diff_time.tv_nsec / (1000 * 1000 * 10));
	markup = g_markup_printf_escaped(format, clock_time);
	gtk_label_set_markup(GTK_LABEL(data->ddisp_widgets[TIMER]), markup);
	g_free(clock_time);
	g_free(markup);

	fprintf(stderr, "Finished the drive, total time: %ld:%ld:%ld\n",
			diff_time.tv_sec / 60,
			diff_time.tv_sec % 60,
			diff_time.tv_nsec / (1000 * 1000 * 10));

	gps_stream(&gps_data, WATCH_DISABLE, NULL);
	gps_close(&gps_data);

	g_object_unref(data->drive_container);

	g_main_context_pop_thread_default(worker_context);

	return NULL;
}
