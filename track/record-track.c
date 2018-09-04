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

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <math.h>
#include <gps.h>
#include <gtk/gtk.h>
#include <osm-gps-map.h>
#include "common.h"
#include "track.h"

static gboolean record_file_save_press_event(GtkWidget *widget,
											GdkEventButton *event,
											gpointer user_data)
{
	gtk_user_data *data = user_data;
	GtkWidget *record_file_save_dialog;
	int res;

	record_file_save_dialog =
			gtk_file_chooser_dialog_new("Choos a track...",
										GTK_WINDOW(data->window),
										GTK_FILE_CHOOSER_ACTION_SAVE,
										"Cancel", GTK_RESPONSE_CANCEL,
										"Save", GTK_RESPONSE_ACCEPT,
										NULL);

	gtk_file_chooser_set_do_overwrite_confirmation(GTK_FILE_CHOOSER(record_file_save_dialog), true);

	res = gtk_dialog_run(GTK_DIALOG(record_file_save_dialog));
	if (res == GTK_RESPONSE_ACCEPT) {
		data->record_track_filepath =
				gtk_file_chooser_get_filename(GTK_FILE_CHOOSER(record_file_save_dialog));

		if (data->record_track_filepath != NULL) {
			data->fd = fopen(data->record_track_filepath, "w+");

			if (data->fd == NULL) {
				fprintf(stderr, "Unable to open GPX file %s for writing\n",
					    data->record_track_filepath);
			} else {
				gtk_button_set_label(GTK_BUTTON(data->record_file_save_button),
									data->record_track_filepath);
				gtk_widget_set_sensitive(data->record_start_button, true);
				/* Set the label as well */
			}
		}
	}

	gtk_widget_destroy(record_file_save_dialog);

	return true;
}

static gboolean record_start_button_press_event(GtkWidget *widget,
												GdkEventButton *event,
												gpointer user_data)
{
	gtk_user_data *data = user_data;

	data->save = !data->save;

	if (data->save) {
		gtk_button_set_label(GTK_BUTTON(data->record_start_button),
							 "Stop Recording");
		gtk_widget_set_sensitive(data->record_back_button, false);
	} else if (!data->save) {
		gtk_button_set_label(GTK_BUTTON(data->record_start_button),
							 "Start Recording");
		gtk_widget_set_sensitive(data->record_back_button, true);
	}

	return false;
}

static gboolean record_finish_button_press_event(GtkWidget *widget,
												GdkEventButton *event,
												gpointer user_data)
{
	gtk_user_data *data = user_data;

	osm_gps_map_track_remove_all((OsmGpsMap *) data->record_map);

	data->record_page = false;
	gtk_container_remove(GTK_CONTAINER(data->window), data->record_container);

	gtk_container_add(GTK_CONTAINER(data->window), data->main_button_box);
	gtk_widget_show_all(data->window);

	/* Do more cleanup */
	if (data->fd) {
		fclose(data->fd);
	}
	g_free(data->record_track_filepath);

	return false;
}

gpointer record_track(gpointer user_data)
{
	gtk_user_data *data = user_data;
	cmd_args args = *data->args;
	OsmGpsMapTrack *osm_track;
	OsmGpsMapPoint *point;
	OsmGpsMap *map = OSM_GPS_MAP(data->record_map);
	
	struct gps_data_t gps_data;
	int ret;

	gps_data = connect_to_gpsd(args);
	gps_stream(&gps_data, WATCH_ENABLE | WATCH_JSON, NULL);

	osm_track = osm_gps_map_track_new();
	osm_gps_map_track_add(map, osm_track);

	fprintf(stderr, "Connected to GPSD and opened track file\n");

	/* Read data and write to file until user interrupts us */
	while (data->record_page) {
		if (gps_waiting(&gps_data, 500)) {
			ret = gps_read(&gps_data);

			if (ret < 0) {
				fprintf(stderr, "gps_read error: %d\n", ret);
				exit(1);
			}

			if (gps_data.set &&
			    !isnan(gps_data.fix.latitude) &&
			    !isnan(gps_data.fix.longitude)) {
				/* Set the current position and zoom from the point */
				osm_gps_map_set_center_and_zoom(map, gps_data.fix.latitude,
												gps_data.fix.longitude,
												12);


				/* Plot current position, something like: osm_gps_map_gps_add() */
				osm_gps_map_gps_add((OsmGpsMap *) data->record_map,
									gps_data.fix.latitude,
									gps_data.fix.longitude,
									gps_data.fix.track);

				if (data->save && data->fd) {
					/* Fix this to be in a real formart */
					fprintf(data->fd, "mode: %d, ", gps_data.fix.mode);
					fprintf(data->fd, "time: %10.0f, ", gps_data.fix.time);
					fprintf(data->fd, "latitude: %f, ", gps_data.fix.latitude);
					fprintf(data->fd, "longitude: %f, ", gps_data.fix.longitude);
					fprintf(data->fd, "altitude: %f, ", gps_data.fix.altitude);
					fprintf(data->fd, "speed: %f, ", gps_data.fix.speed);
					fprintf(data->fd, "track: %f, ", gps_data.fix.track);
					fprintf(data->fd, "pdop: %f", gps_data.dop.pdop);
					fprintf(data->fd, "\n");

					point = osm_gps_map_point_new_degrees(gps_data.fix.latitude,
														  gps_data.fix.longitude);
					osm_gps_map_track_add_point(osm_track, point);
					osm_gps_map_point_free(point);
				}
			}
		}
	}

	fprintf(stderr, "Done!\n");
	fflush(data->fd);
	gps_stream(&gps_data, WATCH_DISABLE, NULL);
	gps_close(&gps_data);

	return NULL;
}

gboolean record_button_press_event(GtkWidget *widget,
				GdkEventButton *event,
				gpointer user_data)
{
	gtk_user_data *data = user_data;
	GtkWidget *vbox = gtk_button_box_new(GTK_ORIENTATION_VERTICAL);

	/* Remove the main container. */
	g_object_ref(data->main_button_box);
	gtk_container_remove(GTK_CONTAINER(data->window), data->main_button_box);

	/* We are on the record page */
	data->record_page = true;

	data->record_container = gtk_paned_new(GTK_ORIENTATION_HORIZONTAL);

	data->record_map = osm_gps_map_new();
	gtk_paned_pack1(GTK_PANED(data->record_container), data->record_map, true, true);

	gtk_paned_pack2(GTK_PANED(data->record_container), vbox, false, false);

	data->record_file_save_button = gtk_button_new_with_label("Choose a file...");
	gtk_box_pack_start(GTK_BOX(vbox), data->record_file_save_button, false, false, 10);
	g_signal_connect(G_OBJECT(data->record_file_save_button), "button-press-event",
			G_CALLBACK(record_file_save_press_event), user_data);

	data->record_start_button = gtk_button_new_with_label("Start Recording");
	gtk_box_pack_start(GTK_BOX(vbox), data->record_start_button, false, false, 10);
	gtk_widget_set_sensitive(data->record_start_button, false);
	g_signal_connect(G_OBJECT(data->record_start_button), "button-press-event",
			G_CALLBACK(record_start_button_press_event), user_data);

	data->record_back_button = gtk_button_new_with_label("Back to main page");
	gtk_box_pack_start(GTK_BOX(vbox), data->record_back_button, false, false, 10);
	g_signal_connect(G_OBJECT(data->record_back_button), "button-press-event",
			G_CALLBACK(record_finish_button_press_event), user_data);

	gtk_button_box_set_layout(GTK_BUTTON_BOX(vbox), GTK_BUTTONBOX_CENTER);

	gtk_container_add(GTK_CONTAINER(data->window), data->record_container);
	gtk_widget_show_all(data->window);

	/* Don't start saving */
	data->save = false;

	while (gtk_events_pending()) {
		gtk_main_iteration();
	}

	data->record_track_thread = g_thread_new("Record Track Thread",
											 record_track,
											 user_data);

	return true;
}
