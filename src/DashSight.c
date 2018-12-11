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

gboolean close_button_press_event(GtkWidget *widget,
								  GdkEventButton *event,
								  gpointer user_data)
{
	gtk_user_data *data = user_data;
	
	gtk_window_close(GTK_WINDOW(data->window));
}

static void activate(GtkApplication* app,
		gpointer user_data)
{
	gtk_user_data *data = user_data;
	GtkCssProvider *cssProvider;
	GdkPixbuf *main_image_pixbuf, *record_button_image_pixbuf;
	GdkPixbuf *drive_line_button_image_pixbuf;
	GtkWidget *main_image, *button_box;
	GtkWidget *record_button_image, *drive_line_button_image;
	GtkWidget *record_button, *drive_line_button, *close_button;

	data->window = gtk_application_window_new(app);
	gtk_window_set_title(GTK_WINDOW(data->window), "Lap Timer");
	gtk_window_fullscreen(GTK_WINDOW(data->window));

	cssProvider = gtk_css_provider_new();
	gtk_css_provider_load_from_path(cssProvider, "theme.css", NULL);
	gtk_style_context_add_provider_for_screen(gdk_screen_get_default(),
									GTK_STYLE_PROVIDER(cssProvider),
									GTK_STYLE_PROVIDER_PRIORITY_USER);


	main_image_pixbuf = gdk_pixbuf_new_from_file("SplashPage.png", NULL);
	main_image_pixbuf = gdk_pixbuf_scale_simple(main_image_pixbuf,
												640, 320,
												GDK_INTERP_BILINEAR);
	main_image = gtk_image_new_from_pixbuf(main_image_pixbuf);

	record_button_image_pixbuf = gdk_pixbuf_new_from_file("RecordTrack.png", NULL);
	record_button_image_pixbuf = gdk_pixbuf_scale_simple(record_button_image_pixbuf,
														60, 60,
														GDK_INTERP_BILINEAR);
	record_button_image = gtk_image_new_from_pixbuf(record_button_image_pixbuf);

	drive_line_button_image_pixbuf = gdk_pixbuf_new_from_file("DriveLine.png", NULL);
	drive_line_button_image_pixbuf = gdk_pixbuf_scale_simple(drive_line_button_image_pixbuf,
														60, 60,
														GDK_INTERP_BILINEAR);
	drive_line_button_image = gtk_image_new_from_pixbuf(drive_line_button_image_pixbuf);

	data->main_page = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
	button_box = gtk_button_box_new(GTK_ORIENTATION_HORIZONTAL);

	record_button = gtk_button_new_with_label("Record new track");
	gtk_container_add(GTK_CONTAINER(button_box), record_button);
	gtk_button_set_always_show_image(GTK_BUTTON(record_button), TRUE);
	gtk_button_set_image(GTK_BUTTON(record_button), record_button_image);
	g_signal_connect(G_OBJECT(record_button), "button-press-event",
			G_CALLBACK(record_button_press_event), user_data);

	drive_line_button = gtk_button_new_with_label("Drive a single line");
	gtk_container_add(GTK_CONTAINER(button_box), drive_line_button);
	gtk_button_set_image(GTK_BUTTON(drive_line_button), drive_line_button_image);
	g_signal_connect(G_OBJECT(drive_line_button), "button-press-event",
			G_CALLBACK(drive_line_button_press_event), user_data);

	close_button = gtk_button_new_with_label("Close!");
	gtk_container_add(GTK_CONTAINER(button_box), close_button);
	g_signal_connect(G_OBJECT(close_button), "button-press-event",
			G_CALLBACK(close_button_press_event), user_data);

	gtk_button_box_set_layout(GTK_BUTTON_BOX(button_box),
								GTK_BUTTONBOX_EXPAND);

	gtk_box_pack_start(GTK_BOX(data->main_page),
						main_image,
						true, true, 0);
	gtk_box_pack_start(GTK_BOX(data->main_page),
						button_box,
						true, true, 0);
	gtk_container_add(GTK_CONTAINER(data->window), data->main_page);

	gtk_widget_show_all(data->window);
}

int main(int argc, char **argv)
{
	GtkApplication *app;
	cmd_args *args = g_new0(cmd_args, 1);
	gtk_user_data *data = g_new0(gtk_user_data, 1);
	int status = 0;

	data->args = args;

	app = gtk_application_new("org.alistair23.DashSight", G_APPLICATION_FLAGS_NONE);
	g_signal_connect(app, "activate", G_CALLBACK (activate), (gpointer) data);
	status = g_application_run(G_APPLICATION(app), argc, argv);
	g_object_unref(app);

	return status;
}
