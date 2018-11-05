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

static void drive_file_load_file_set_event(GtkFileChooserButton *widget,
											gpointer user_data)
{
	gtk_user_data *data = user_data;

	data->drive_track_filepath =
			gtk_file_chooser_get_filename(GTK_FILE_CHOOSER(data->drive_file_load));
	data->drive_track_updated = true;
}

static gboolean drive_file_load_file_press_event(GtkWidget *widget,
												GdkEventButton *event,
												gpointer user_data)
{
	const char *start_time = "0:00:00";
	const char *format = TIMER_FORMAT;
	char *markup;
	gtk_user_data *data = user_data;
	track *cur_track = data->loaded_track;

	gtk_container_remove(GTK_CONTAINER(data->window), data->drive_container);

	data->drive_container = gtk_grid_new();
	gtk_container_add(GTK_CONTAINER(data->window), data->drive_container);

	gtk_grid_set_row_spacing(GTK_GRID(data->drive_container), 100);
	gtk_grid_set_column_spacing(GTK_GRID(data->drive_container), 40);

	data->drive_map = osm_gps_map_new();
	if (cur_track) {
		osm_gps_map_set_center_and_zoom(OSM_GPS_MAP(data->drive_map), cur_track->start.lat, cur_track->start.lon, MAP_ZOOM_LEVEL);
		osm_gps_map_track_add(OSM_GPS_MAP(data->drive_map), cur_track->osm_track);
	}
	gtk_grid_attach(GTK_GRID(data->drive_container), data->drive_map, 0, 0, 12, 6);

	data->timer_display = gtk_label_new(NULL);
	markup = g_markup_printf_escaped(format, start_time);
	gtk_label_set_markup(GTK_LABEL(data->timer_display), markup);
	gtk_grid_attach(GTK_GRID(data->drive_container), data->timer_display, 12, 0, 1, 1);
	g_free(markup);

	gtk_widget_show_all(data->window);

	while (gtk_events_pending()) {
		gtk_main_iteration();
	}

	data->load_page = false;
}

gpointer obdii_data(gpointer user_data)
{
	gtk_user_data *data = user_data;

	return NULL;
}

gpointer drive_line(gpointer user_data)
{
	gtk_user_data *data = user_data;
	cmd_args args = *data->args;
	FILE *fd;
	struct gps_data_t gps_data;
	struct timespec cur_time, diff_time;
	track *cur_track;
	OsmGpsMap *map = OSM_GPS_MAP(data->drive_map);
	int ret;
	gchar *clock_time;
	const char *format = TIMER_FORMAT;
	char *markup;

	gps_data = connect_to_gpsd(args);
	gps_stream(&gps_data, WATCH_ENABLE | WATCH_JSON, NULL);

	while (data->load_page) {
		if (data->drive_track_filepath && data->drive_track_updated) {
			osm_gps_map_track_remove_all(map);

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

	/* Poll until we hit the start line */
	while (1) {
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
					clock_gettime(CLOCK_MONOTONIC_RAW, &cur_track->start.time);
					break;
				}
			}
		}
	}

	fprintf(stderr, "Starting the drive\n");

	/* Poll until we hit the end line and do stuff */
	while (1) {
		clock_gettime(CLOCK_MONOTONIC_RAW, &cur_time);
		diff_time = timeval_subtract(&cur_time, &cur_track->start.time);
		clock_time = g_strdup_printf("%ld:%ld:%ld\r",
									diff_time.tv_sec, diff_time.tv_nsec / 1000000,
									(diff_time.tv_nsec / 1000) % 1000);
		markup = g_markup_printf_escaped(format, clock_time);
		gtk_label_set_markup(GTK_LABEL(data->timer_display), markup);
		g_free(clock_time);
		g_free(markup);
		if (gps_waiting(&gps_data, 10)) {
			ret = gps_read(&gps_data, NULL, 0);

			if (ret < 0) {
				fprintf(stderr, "gps_read error: %d\n", ret);
				exit(1);
			}

			if (equal(gps_data.fix.latitude, cur_track->end.lat, 0.0005) &&
				equal(gps_data.fix.longitude, cur_track->end.lon, 0.0005)) {
				clock_gettime(CLOCK_MONOTONIC_RAW, &cur_track->end.time);
				diff_time = timeval_subtract(&cur_track->end.time, &cur_track->start.time);
				break;
			}
		}
	}

	clock_gettime(CLOCK_MONOTONIC_RAW, &cur_time);
	diff_time = timeval_subtract(&cur_time, &cur_track->start.time);
	clock_time = g_strdup_printf("%ld:%ld:%ld\r",
								diff_time.tv_sec, diff_time.tv_nsec / 1000000,
								(diff_time.tv_nsec / 1000) % 1000);
	markup = g_markup_printf_escaped(format, clock_time);
	gtk_label_set_markup(GTK_LABEL(data->timer_display), markup);
	g_free(clock_time);
	g_free(markup);

	fprintf(stderr, "Finished the drive, total time: %ld:%ld:%ld\n",
			diff_time.tv_sec, diff_time.tv_nsec / 1000000,
			(diff_time.tv_nsec / 1000) % 1000);

	gps_stream(&gps_data, WATCH_DISABLE, NULL);
	gps_close(&gps_data);

	return NULL;
}

gboolean drive_line_button_press_event(GtkWidget *widget,
				GdkEventButton *event,
				gpointer user_data)
{
	gtk_user_data *data = user_data;
	GtkWidget *vbox = gtk_button_box_new(GTK_ORIENTATION_VERTICAL);

	/* Remove the main container. */
	g_object_ref(data->main_page);
	gtk_container_remove(GTK_CONTAINER(data->window), data->main_page);

	data->drive_container = gtk_paned_new(GTK_ORIENTATION_HORIZONTAL);

	data->drive_map = osm_gps_map_new();
	gtk_paned_pack1(GTK_PANED(data->drive_container), data->drive_map, true, true);

	gtk_paned_pack2(GTK_PANED(data->drive_container), vbox, false, false);

	data->drive_file_load =
			gtk_file_chooser_button_new("Load a track...",
										GTK_FILE_CHOOSER_ACTION_OPEN);
	gtk_box_pack_start(GTK_BOX(vbox), data->drive_file_load, false, false, 10);
	g_signal_connect(G_OBJECT(data->drive_file_load), "file-set",
			G_CALLBACK(drive_file_load_file_set_event), user_data);

	data->drive_file_load_button = gtk_button_new_with_label("Load this file");
	gtk_box_pack_start(GTK_BOX(vbox), data->drive_file_load_button, false, false, 10);
	g_signal_connect(G_OBJECT(data->drive_file_load_button), "button-press-event",
			G_CALLBACK(drive_file_load_file_press_event), user_data);

	gtk_button_box_set_layout(GTK_BUTTON_BOX(vbox), GTK_BUTTONBOX_CENTER);

	gtk_container_add(GTK_CONTAINER(data->window), data->drive_container);
	gtk_widget_show_all(data->window);

	while (gtk_events_pending()) {
		gtk_main_iteration();
	}

	/* First we need to load a track */
	data->load_page = true;
	data->drive_track_updated = false;

	data->drive_track_thread = g_thread_new("Drive Thread",
											 drive_line,
											 user_data);
	data->obdii_thread = g_thread_new("OBDII Data Thread",
									  obdii_data,
									  user_data);

	return true;
}
