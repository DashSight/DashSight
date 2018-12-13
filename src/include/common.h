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

#include <Python.h>
#include <stdbool.h>
#include <gtk/gtk.h>
#include <gps.h>
#include <osm-gps-map.h>
#include "track.h"

typedef struct cmd_args {
	enum { NONE, GUI, RECORD_TRACK, CIRC_DRIVE, SINGLE_DRIVE } mode;
	char *server;
	char *port;
	char *gpx;
} cmd_args;

typedef struct track track;

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
	GtkWidget *load_drive_container;
	GtkWidget *drive_container;
	GMainLoop *obdii_loop, *drive_loop;
	GtkWidget *drive_map;
	GtkWidget *drive_grid;
	GtkWidget *drive_file_load, *drive_file_load_button;
	GtkWidget *return_home;
	GtkWidget *timer_display;
	GtkWidget *coolant_temp_disp;
	GtkWidget *intake_temp_disp;
	GtkWidget *throttle_bar;
	GtkWidget *taco_draw_area;
	GThread *drive_track_thread, *obdii_thread;
	int revs;
	void *loaded_track;
	bool load_page, drive_track_updated;
	bool finished_drive;
} gtk_user_data;

typedef struct drive_loop_data
{
	gtk_user_data *data;

	struct gps_data_t gps_data;
	struct timespec *start_time;
	OsmGpsMap *map;
	track *cur_track;
} drive_loop_data;

typedef struct obdii_loop_data
{
	gtk_user_data *data;

	PyObject *pModule;
} obdii_loop_data;

#define TIMER_FORMAT "<span font_desc=\"55.0\">\%s</span>"

#define COOLANT_FORMAT "<span font_desc=\"25.0\" foreground=\"green\">\%s</span>"
#define INTAKE_FORMAT "<span font_desc=\"25.0\" foreground=\"yellow\">\%s</span>"

#define ARRAY_SIZE(x) (sizeof(x) / sizeof((x)[0]))

#define REV_ANGLE(x) (((x / 7000.0) * M_PI) + ((M_PI / 3.0) * 2.5))

struct gps_data_t connect_to_gpsd(cmd_args args);

gboolean taco_draw_callback(GtkWidget *widget, cairo_t *cr, gpointer user_data);
gpointer prepare_to_drive(gpointer user_data);

bool equal(float a, float b, float epsilon);

struct timespec timeval_subtract(struct timespec *x, struct timespec *y);

#endif /* COMMON_H */
