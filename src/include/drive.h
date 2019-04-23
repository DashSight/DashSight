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

#ifndef DRIVE_H
#define DRIVE_H

#include <Python.h>
#include <stdbool.h>
#include <gtk/gtk.h>
#include <gps.h>
#include <osm-gps-map.h>
#include "common.h"
#include "track.h"

typedef struct drive_loop_data
{
	gtk_user_data *data;

	struct gps_data_t gps_data;
	struct timespec *start_time;
	struct timespec best_time;
	OsmGpsMap *map;
	track *cur_track;
} drive_loop_data;

enum gtk_type_enum {
	DRIVE_PROGRESS_BAR,
	DRIVE_LABEL
} gtk_type_enum;

typedef struct drive_display {
	enum drive_disp_type type;
	enum gtk_type_enum gtk_type;
	const char *name;
	const char *zero;
	const char *context_name;
	const char *format;
	int start_x;
	int start_y;
} drive_display;

#define LOCATION_MARGIN 0.00005

#endif /* DRIVE_H; */
