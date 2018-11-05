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

#ifndef COMMON_H
#define COMMON_H

#include <gtk/gtk.h>
#include <osm-gps-map.h>

typedef struct cmd_args {
	enum { NONE, GUI, RECORD_TRACK, CIRC_DRIVE, SINGLE_DRIVE } mode;
	char *server;
	char *port;
	char *gpx;
} cmd_args;

typedef struct gtk_user_data
{
	cmd_args *args;
	FILE *fd;

	GtkWidget *window;

	/* Main page */
	GtkWidget *main_page;

	/* Record Track */
	gchar *record_track_filepath;
	GtkWidget *record_container;
	GtkWidget *record_map;
	GtkWidget *record_start_button, *record_back_button;
	GtkWidget *record_file_save_button;
	GThread *record_track_thread;
	bool save, record_page;

	/* Drive Track */
	gchar *drive_track_filepath;
	GtkWidget *drive_container;
	GtkWidget *drive_map;
	GtkWidget *drive_grid;
	GtkWidget *drive_file_load, *drive_file_load_button;
	GtkWidget *timer_display;
	GThread *drive_track_thread;
	void *loaded_track;
	bool load_page, drive_track_updated;
} gtk_user_data;

#define TIMER_FORMAT "<span font_desc=\"55.0\">\%s</span>"

struct gps_data_t connect_to_gpsd(cmd_args args);

gpointer drive_line(gpointer user_data);

bool equal(float a, float b, float epsilon);

struct timespec timeval_subtract(struct timespec *x, struct timespec *y);

#endif /* COMMON_H */
