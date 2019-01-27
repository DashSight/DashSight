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

static void print_gpx_start(FILE *fd)
{
	fprintf(fd, "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
	fprintf(fd, "<gpx version=\"1.1\" creator=\"DashSight\"\n");
	fprintf(fd,
	"        xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n");
	fprintf(fd, "        xmlns=\"http://www.topografix.com/GPX/1.1\"\n");
	fprintf(fd,
	"        xsi:schemaLocation=\"http://www.topografix.com/GPS/1/1\n");
	fprintf(fd, "        http://www.topografix.com/GPX/1/1/gpx.xsd\">\n");
	fflush(fd);
}

static void print_gpx_stop(FILE *fd)
{
	fprintf(fd, "</gpx>\n");
	fflush(fd);
}

static void print_gpx_metadata(FILE *fd)
{
	fprintf(fd, "  <metadata>\n");
	fprintf(fd, "    <link href=\"https://github.com/alistair23/DashSight\">\n");
	fprintf(fd, "      <text>DashSight</text>\n");
	fprintf(fd, "    </link>\n");
	fprintf(fd, "  </metadata>\n");
	fflush(fd);
}


static void print_gpx_track_start(FILE *fd, char* track_name)
{
	fprintf(fd, "  <trk>\n");
	fprintf(fd, "    <name>%s</name>\n", track_name);
	fflush(fd);
}

static void print_gpx_track_stop(FILE *fd)
{
	fprintf(fd, "    </trkseg>\n");
	fprintf(fd, "  </trk>\n");
	fflush(fd);
}

static void print_gpx_track_seg_start(FILE *fd)
{
	fprintf(fd, "    <trkseg>\n");
	fflush(fd);
}

static void print_gpx_track_seg_stop(FILE *fd)
{
	fprintf(fd, "    </trkseg>\n");
	fflush(fd);
}

static gboolean record_file_save_press_event(GtkWidget *widget,
											GdkEventButton *event,
											gpointer user_data)
{
	gtk_user_data *data = user_data;
	GtkWidget *record_file_save_dialog;
	int res;
	gchar *filename;

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
				filename = g_path_get_basename(data->record_track_filepath);

				print_gpx_start(data->fd);
				print_gpx_metadata(data->fd);
				print_gpx_track_start(data->fd, filename);

				gtk_button_set_label(GTK_BUTTON(data->record_file_save_button),
									filename);
				gtk_widget_set_sensitive(data->record_start_button, true);

				g_free(filename);
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

		print_gpx_track_seg_start(data->fd);
	} else if (!data->save) {
		gtk_button_set_label(GTK_BUTTON(data->record_start_button),
							 "Start Recording");
		gtk_widget_set_sensitive(data->record_back_button, true);

		print_gpx_track_seg_stop(data->fd);
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

	gtk_container_add(GTK_CONTAINER(data->window), data->main_page);
	gtk_widget_show_all(data->window);

	/* Do more cleanup */
	if (data->fd) {
		print_gpx_track_stop(data->fd);
		print_gpx_stop(data->fd);
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
	char tbuf[128];

	struct gps_data_t gps_data;
	int ret;

	gps_data = connect_to_gpsd(args);
	gps_stream(&gps_data, WATCH_ENABLE | WATCH_JSON, NULL);

	osm_track = osm_gps_map_track_new();
	osm_gps_map_track_add(map, osm_track);

	g_object_set(G_OBJECT(data->record_map),
			"record-trip-history", false,
			NULL);

	fprintf(stderr, "Connected to GPSD and opened track file\n");

	/* Read data and write to file until user interrupts us */
	while (data->record_page) {
		if (gps_waiting(&gps_data, 500)) {
			ret = gps_read(&gps_data, NULL, 0);

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
												MAP_ZOOM_LEVEL);


				/* Plot current position, something like: osm_gps_map_gps_add() */
				osm_gps_map_gps_add(map,
									gps_data.fix.latitude,
									gps_data.fix.longitude,
									gps_data.fix.track);

				if (data->save && data->fd) {
					fprintf(data->fd, "      <trkpt lat=\"%f\" lon=\"%f\">\n",
							gps_data.fix.latitude,
							gps_data.fix.longitude);
					fprintf(data->fd, "        <ele>%.2git f</ele>\n",
							gps_data.fix.altitude);
					fprintf(data->fd, "        <time>%s</time>\n",
							unix_to_iso8601(gps_data.fix.time, tbuf, sizeof(tbuf)));
					fprintf(data->fd, "      </trkpt>\n");

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
	g_object_ref(data->main_page);
	gtk_container_remove(GTK_CONTAINER(data->window), data->main_page);

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
