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
#include "obdii_commands.h"

gboolean drive_loop(gpointer user_data)
{
	drive_loop_data *drive_data = user_data;
	gtk_user_data *data = drive_data->data;
	struct gps_data_t gps_data = drive_data->gps_data;
	struct timespec *start_time = drive_data->start_time;
	OsmGpsMap *map = drive_data->map;
	track *cur_track = drive_data->cur_track;

	struct timespec cur_time, diff_time;
	gchar *clock_time;
	const char *format = TIMER_FORMAT;
	char *markup;
	int ret;

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
	gtk_label_set_markup(GTK_LABEL(data->timer_display), markup);
	g_free(clock_time);
	g_free(markup);
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

			if (!cur_track) {
				/* We don't have a map loaded */
				osm_gps_map_set_center_and_zoom(map,
							gps_data.fix.latitude,
							gps_data.fix.longitude,
							MAP_ZOOM_LEVEL);
			} else if (cur_track &&
				equal(gps_data.fix.latitude, cur_track->end.lat, 0.0005) &&
				equal(gps_data.fix.longitude, cur_track->end.lon, 0.0005)) {
				clock_gettime(CLOCK_MONOTONIC_RAW, &cur_track->end.time);
				diff_time = timeval_subtract(&cur_track->end.time, start_time);

				g_mutex_lock(&data->data_mutex);
				g_cond_signal(&data->finished_drive_cond);
				data->finished_drive = true;
				g_mutex_unlock(&data->data_mutex);
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
	track *cur_track;
	struct timespec *start_time;
	OsmGpsMap *map = OSM_GPS_MAP(data->drive_map);
	int ret, pid;
	GSource *source;
	gchar *clock_time;
	const char *format = TIMER_FORMAT;
	char *markup;

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

	source = g_timeout_source_new(10);
	g_source_set_callback(source, drive_loop, drive_data, NULL);
	pid = g_source_attach(source, g_main_context_get_thread_default());

	g_mutex_lock(&data->data_mutex);

	/* Poll until we hit the end line and do stuff */
	while (!data->finished_drive) {
		g_cond_wait(&data->finished_drive_cond, &data->data_mutex);
	}

	g_mutex_unlock(&data->data_mutex);
	g_source_remove(pid);
	g_free(drive_data);

	clock_gettime(CLOCK_MONOTONIC_RAW, &cur_time);
	diff_time = timeval_subtract(&cur_time, start_time);
	clock_time = g_strdup_printf("%02ld:%02ld:%02ld",
								diff_time.tv_sec / 60,
								diff_time.tv_sec % 60,
								diff_time.tv_nsec / (1000 * 1000 * 10));
	markup = g_markup_printf_escaped(format, clock_time);
	gtk_label_set_markup(GTK_LABEL(data->timer_display), markup);
	g_free(clock_time);
	g_free(markup);

	fprintf(stderr, "Finished the drive, total time: %ld:%ld:%ld\n",
			diff_time.tv_sec / 60,
			diff_time.tv_sec % 60,
			diff_time.tv_nsec / (1000 * 1000 * 10));

	gps_stream(&gps_data, WATCH_DISABLE, NULL);
	gps_close(&gps_data);

	g_object_unref(data->drive_container);

	return NULL;
}
