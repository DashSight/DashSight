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

#ifndef TRACK_H
#define TRACK_H

#include <gtk/gtk.h>
#include <osm-gps-map.h>
#include "common.h"

#define MAP_ZOOM_LEVEL 15

typedef struct track_info
{
	float lon, lat;
	struct timespec time;
} track_info;

typedef struct track
{
	track_info start, end;
	bool loop;

	OsmGpsMapTrack *osm_track;
} track;

gpointer record_track(gpointer data);
track *load_track(char *file, bool loop);

gboolean record_button_press_event(GtkWidget *widget,
				GdkEventButton *event,
				gpointer user_data);
gboolean drive_line_button_press_event(GtkWidget *widget,
				GdkEventButton *event,
				gpointer user_data);

#endif /* TRACK_H */
