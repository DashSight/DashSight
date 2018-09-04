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
#include <gps.h>
#include <math.h>
#include <gtk/gtk.h>
#include <osm-gps-map.h>
#include "common.h"
#include "track.h"
#include "arg-parser.h"

static void activate(GtkApplication* app,
		gpointer user_data)
{
	gtk_user_data *data = user_data;
	GtkWidget *record_button, *drive_line_button;

	data->window = gtk_application_window_new(app);
	gtk_window_set_title(GTK_WINDOW(data->window), "Lap Timer");
	gtk_widget_set_size_request(data->window, 800, 680);

	data->main_button_box = gtk_button_box_new(GTK_ORIENTATION_HORIZONTAL);
	gtk_container_add(GTK_CONTAINER(data->window), data->main_button_box);

	record_button = gtk_button_new_with_label("Record new track");
	gtk_container_add(GTK_CONTAINER(data->main_button_box), record_button);
	g_signal_connect(G_OBJECT(record_button), "button-press-event",
			G_CALLBACK(record_button_press_event), user_data);

	drive_line_button = gtk_button_new_with_label("Drive a single line");
	gtk_container_add(GTK_CONTAINER(data->main_button_box), drive_line_button);
	g_signal_connect(G_OBJECT(drive_line_button), "button-press-event",
			G_CALLBACK(drive_line_button_press_event), user_data);

	gtk_button_box_set_layout(GTK_BUTTON_BOX(data->main_button_box),
								GTK_BUTTONBOX_EXPAND);

	gtk_widget_show_all(data->window);
}

int main(int argc, char **argv)
{
	GtkApplication *app;
	cmd_args *args = g_new0(cmd_args, 1);
	gtk_user_data *data = g_new0(gtk_user_data, 1);
	int status = 0;

	args->mode = NONE;
	args->server = NULL;
	args->port = NULL;
	args->gpx = NULL;

	argp_parse(&argp, argc, argv, 0, 0, args);

	if (args->mode == NONE) {
		fprintf(stderr, "You need to specify a mode\n");
		exit(1);
	}

	/* Do more argument error checking */

	data->args = args;

	if (args->mode == GUI) {
		app = gtk_application_new("org.alistair23.lap-timer", G_APPLICATION_FLAGS_NONE);
		g_signal_connect(app, "activate", G_CALLBACK (activate), (gpointer) data);
		/* It's probably best to just use Glib for arg parsing */
		status = g_application_run(G_APPLICATION(app), 1, argv);
		g_object_unref(app);
	} else if (args->mode == RECORD_TRACK) {
		record_track(data);
	} else if (args->mode == SINGLE_DRIVE) {
		drive_line(data);
	}

	return status;
}
