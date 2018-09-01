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
}

static gboolean record_cancel_button_press_event(GtkWidget *widget,
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

	return false;
}

gpointer record_track(gpointer user_data)
{
	gtk_user_data *data = user_data;
	cmd_args args = *data->args;
	OsmGpsMapTrack *osm_track;
	OsmGpsMapPoint *point;
	FILE *fd;
	struct gps_data_t gps_data;
	int ret;

	gps_data = connect_to_gpsd(args);

	fd = fopen(args.gpx, "w+");

	if (fd == NULL) {
		fprintf(stderr, "Unable to open GPX file %s for reading\n",
			    args.gpx);
		exit(-1);
	}

	gps_stream(&gps_data, WATCH_ENABLE | WATCH_JSON, NULL);

	osm_track = osm_gps_map_track_new();
	osm_gps_map_track_add((OsmGpsMap *) data->record_map, osm_track);

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

				/* Plot current position, something like: osm_gps_map_gps_add() */
				osm_gps_map_gps_add((OsmGpsMap *) data->record_map,
									gps_data.fix.latitude,
									gps_data.fix.longitude,
									gps_data.fix.track);

				if (data->save) {
					/* Fix this to be in a real formart */
					fprintf(fd, "mode: %d, ", gps_data.fix.mode);
					fprintf(fd, "time: %10.0f, ", gps_data.fix.time);
					fprintf(fd, "latitude: %f, ", gps_data.fix.latitude);
					fprintf(fd, "longitude: %f, ", gps_data.fix.longitude);
					fprintf(fd, "altitude: %f, ", gps_data.fix.altitude);
					fprintf(fd, "speed: %f, ", gps_data.fix.speed);
					fprintf(fd, "track: %f, ", gps_data.fix.track);
					fprintf(fd, "pdop: %f", gps_data.dop.pdop);
					fprintf(fd, "\n");
					/* At the moment there is no way to niceley
					 * exit this loop so we have to flush on each
					 * loop. Remove this when it gets nicer.
					 */
					fflush(fd);

					point = osm_gps_map_point_new_degrees(gps_data.fix.latitude,
														  gps_data.fix.longitude);
					osm_gps_map_track_add_point(osm_track, point);
					osm_gps_map_point_free(point);
				}
			}
		}
	}

	fprintf(stderr, "Done!\n");

	fclose(fd);
	gps_stream(&gps_data, WATCH_DISABLE, NULL);
	gps_close(&gps_data);
}

gboolean record_button_press_event(GtkWidget *widget,
				GdkEventButton *event,
				gpointer user_data)
{
	gtk_user_data *data = user_data;
	GtkWidget *vbox = gtk_box_new(true, 10);

	/* Remove the main container. */
	gtk_window_set_has_user_ref_count(GTK_WINDOW(data->main_button_box), true);
	gtk_container_remove(GTK_CONTAINER(data->window), data->main_button_box);

	/* We are on the record page */
	data->record_page = true;

	data->record_container = gtk_box_new(false, 100);

	data->record_map = osm_gps_map_new();
	gtk_box_pack_start(GTK_BOX(data->record_container), data->record_map, true, true, 10);

	gtk_box_pack_start(GTK_BOX(data->record_container), vbox, true, true, 10);

	data->record_start_button = gtk_button_new_with_label("Start Recording");
	gtk_box_pack_start(GTK_BOX(vbox), data->record_start_button, true, true, 10);
	g_signal_connect(G_OBJECT(data->record_start_button), "button-press-event",
			G_CALLBACK(record_start_button_press_event), user_data);

	data->record_back_button = gtk_button_new_with_label("Back to main page");
	gtk_box_pack_start(GTK_BOX(vbox), data->record_back_button, true, true, 10);
	g_signal_connect(G_OBJECT(data->record_back_button), "button-press-event",
			G_CALLBACK(record_cancel_button_press_event), user_data);

	gtk_box_set_homogeneous(GTK_BOX(data->record_container), false);

	gtk_container_add(GTK_CONTAINER(data->window), data->record_container);
	gtk_widget_show_all(data->window);

	/* Don't start saving */
	data->save = false;

	data->record_track_thread = g_thread_new("Record Track Thread",
											 record_track,
											 user_data);

	return false;
}
