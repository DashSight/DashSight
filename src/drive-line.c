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

static void drive_file_load_file_set_event(GtkFileChooserButton *widget,
											gpointer user_data)
{
	gtk_user_data *data = user_data;

	data->drive_track_filepath =
			gtk_file_chooser_get_filename(GTK_FILE_CHOOSER(data->drive_file_load));
	data->drive_track_updated = true;
}

gboolean drive_line_return(GtkWidget *widget,
				GdkEventButton *event,
				gpointer user_data)
{
	gtk_user_data *data = user_data;

	g_mutex_lock(&data->data_mutex);
	g_cond_signal(&data->finished_drive_cond);
	data->finished_drive = true;
	g_mutex_unlock(&data->data_mutex);

	g_thread_join(data->obdii_thread);
	g_thread_join(data->drive_track_thread);

	gtk_container_remove(GTK_CONTAINER(data->window), data->drive_container);

	gtk_container_add(GTK_CONTAINER(data->window), data->main_page);
	gtk_widget_show_all(data->window);

	return true;
}

static gboolean drive_file_download_file_press_event(GtkWidget *widget,
												GdkEventButton *event,
												gpointer user_data)
{
	gtk_user_data *data = user_data;
	track *cur_track = data->loaded_track;
	GSList *list;
	OsmGpsMapPoint *first_point, *last_point;

	if (!cur_track) {
		return true;
	}

	list = osm_gps_map_track_get_points(cur_track->osm_track);

	first_point = g_slist_nth_data(list, 0);
	last_point = g_slist_nth_data(list, g_slist_length(list));

	gtk_button_set_label(GTK_BUTTON(data->drive_file_download_button),
						"Downloading");

	osm_gps_map_download_maps(OSM_GPS_MAP(data->drive_map),
							first_point,
							last_point,
							MAP_ZOOM_LEVEL + 3,
							MAP_ZOOM_LEVEL - 3);

	return true;
}

static gboolean drive_file_load_file_press_event(GtkWidget *widget,
												GdkEventButton *event,
												gpointer user_data)
{
	const char *start_time = "00:00:00";
	const char *temp = "0 *C";
	const char *format = TIMER_FORMAT;
	char *markup;
	gtk_user_data *data = user_data;
	track *cur_track = data->loaded_track;

	gtk_container_remove(GTK_CONTAINER(data->window), data->load_drive_container);

	data->drive_container = gtk_grid_new();
	gtk_container_add(GTK_CONTAINER(data->window), data->drive_container);

	gtk_grid_set_row_spacing(GTK_GRID(data->drive_container), 100);
	gtk_grid_set_column_spacing(GTK_GRID(data->drive_container), 40);

	data->drive_map = osm_gps_map_new();
	if (cur_track) {
		osm_gps_map_set_center_and_zoom(OSM_GPS_MAP(data->drive_map), cur_track->start.lat, cur_track->start.lon, MAP_ZOOM_LEVEL);
		osm_gps_map_track_add(OSM_GPS_MAP(data->drive_map), cur_track->osm_track);
	}
	gtk_grid_attach(GTK_GRID(data->drive_container), data->drive_map, 0, 1, 15, 5);

	data->timer_display = gtk_label_new(NULL);
	markup = g_markup_printf_escaped(format, start_time);
	gtk_label_set_markup(GTK_LABEL(data->timer_display), markup);
	gtk_grid_attach(GTK_GRID(data->drive_container), data->timer_display, 0, 0, 10, 1);
	g_free(markup);

	data->throttle_bar = gtk_progress_bar_new();
	gtk_grid_attach(GTK_GRID(data->drive_container), data->throttle_bar, 15, 0, 5, 1);

	data->taco_draw_area = gtk_drawing_area_new();
	gtk_widget_set_size_request(data->taco_draw_area, 100, 100);
	gtk_grid_attach(GTK_GRID(data->drive_container), data->taco_draw_area, 10, 0, 5, 2);
	g_signal_connect(G_OBJECT(data->taco_draw_area), "draw",
					G_CALLBACK(taco_draw_callback), data);

	data->coolant_temp_disp = gtk_label_new(NULL);
	format = COOLANT_FORMAT;
	markup = g_markup_printf_escaped(format, temp);
	gtk_label_set_markup(GTK_LABEL(data->coolant_temp_disp), markup);
	gtk_grid_attach(GTK_GRID(data->drive_container), data->coolant_temp_disp, 15, 1, 1, 1);
	g_free(markup);

	data->intake_temp_disp = gtk_label_new(NULL);
	format = INTAKE_FORMAT;
	markup = g_markup_printf_escaped(format, temp);
	gtk_label_set_markup(GTK_LABEL(data->intake_temp_disp), markup);
	gtk_grid_attach(GTK_GRID(data->drive_container), data->intake_temp_disp, 16, 1, 1, 1);
	g_free(markup);

	data->return_home = gtk_button_new_with_label("Return");
	gtk_grid_attach(GTK_GRID(data->drive_container), data->return_home, 15, 3, 1, 1);
	g_signal_connect(G_OBJECT(data->return_home), "button-press-event",
			G_CALLBACK(drive_line_return), user_data);

	gtk_widget_show_all(data->window);

	while (gtk_events_pending()) {
		gtk_main_iteration();
	}

	data->load_page = false;

	return false;
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

	data->load_drive_container = gtk_paned_new(GTK_ORIENTATION_HORIZONTAL);

	data->drive_map = osm_gps_map_new();
	gtk_paned_pack1(GTK_PANED(data->load_drive_container), data->drive_map, true, true);

	gtk_paned_pack2(GTK_PANED(data->load_drive_container), vbox, false, false);

	data->drive_file_load =
			gtk_file_chooser_button_new("Load a track...",
										GTK_FILE_CHOOSER_ACTION_OPEN);
	gtk_box_pack_start(GTK_BOX(vbox), data->drive_file_load, false, false, 10);
	g_signal_connect(G_OBJECT(data->drive_file_load), "file-set",
			G_CALLBACK(drive_file_load_file_set_event), user_data);

	data->drive_file_download_button = gtk_button_new_with_label("Download this map");
	gtk_box_pack_start(GTK_BOX(vbox), data->drive_file_download_button, false, false, 10);
	g_signal_connect(G_OBJECT(data->drive_file_download_button), "button-press-event",
			G_CALLBACK(drive_file_download_file_press_event), user_data);

	data->drive_file_load_button = gtk_button_new_with_label("Load this file");
	gtk_box_pack_start(GTK_BOX(vbox), data->drive_file_load_button, false, false, 10);
	g_signal_connect(G_OBJECT(data->drive_file_load_button), "button-press-event",
			G_CALLBACK(drive_file_load_file_press_event), user_data);

	gtk_button_box_set_layout(GTK_BUTTON_BOX(vbox), GTK_BUTTONBOX_CENTER);

	gtk_container_add(GTK_CONTAINER(data->window), data->load_drive_container);
	gtk_widget_show_all(data->window);

	while (gtk_events_pending()) {
		gtk_main_iteration();
	}

	/* First we need to load a track */
	data->load_page = true;
	data->drive_track_updated = false;
	data->finished_drive = false;

	data->drive_track_thread = g_thread_new("Drive Thread",
											 prepare_to_drive,
											 user_data);
	data->obdii_thread = g_thread_new("OBDII Data Thread",
									  obdii_start_connection,
									  user_data);

	return true;
}
