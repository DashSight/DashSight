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

typedef struct cmd_args {
	enum { NONE, GUI, RECORD_TRACK, CIRC_DRIVE, SINGLE_DRIVE } mode;
	char *server;
	char *port;
	char *gpx;
} cmd_args;

typedef struct gtk_user_data
{
	cmd_args *args;

	GtkWidget *window;

	/* Main page */
	GtkWidget *main_button_box;

	/* Record Track */
	GtkWidget *record_map;

	GThread *record_track_thread;
} gtk_user_data;

struct gps_data_t connect_to_gpsd(cmd_args args);

void drive_line(cmd_args args);

bool equal(float a, float b, float epsilon);

struct timespec timeval_subtract(struct timespec *x, struct timespec *y);

#endif /* COMMON_H */
