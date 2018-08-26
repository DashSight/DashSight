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
#include "common.h"

void drive_line(cmd_args args)
{
	FILE *fd;
	struct gps_data_t gps_data;
	struct timespec cur_time, diff_time;
	track cur_track;
	int ret;

	cur_track = load_track(args.gpx, false);

	gps_data = connect_to_gpsd(args);
	gps_stream(&gps_data, WATCH_ENABLE | WATCH_JSON, NULL);

	/* Poll until we hit the start line */
	while (1) {
		if (gps_waiting(&gps_data, 500)) {
			ret = gps_read(&gps_data);

			if (ret < 0) {
				fprintf(stderr, "gps_read error: %d\n", ret);
				exit(1);
			}

			if (equal(gps_data.fix.latitude, cur_track.start.lat, 0.0005) &&
				equal(gps_data.fix.longitude, cur_track.start.lon, 0.0005)) {
				clock_gettime(CLOCK_MONOTONIC_RAW, &cur_track.start.time);
				break;
			}
		}
	}

	fprintf(stderr, "Starting the drive\n");

	/* Poll until we hit the end line and do stuff */
	while (1) {
		clock_gettime(CLOCK_MONOTONIC_RAW, &cur_time);
		diff_time = timeval_subtract(&cur_time, &cur_track.start.time);
		printf("Time: %ld:%ld:%ld\r",
			diff_time.tv_sec, diff_time.tv_nsec / 1000000,
			(diff_time.tv_nsec / 1000) % 1000);
		fflush(stdout);
		if (gps_waiting(&gps_data, 10)) {
			ret = gps_read(&gps_data);

			if (ret < 0) {
				fprintf(stderr, "gps_read error: %d\n", ret);
				exit(1);
			}

			if (equal(gps_data.fix.latitude, cur_track.end.lat, 0.0005) &&
				equal(gps_data.fix.longitude, cur_track.end.lon, 0.0005)) {
				clock_gettime(CLOCK_MONOTONIC_RAW, &cur_track.end.time);
				diff_time = timeval_subtract(&cur_track.end.time, &cur_track.start.time);
				break;
			}
		}
	}

	fprintf(stderr, "Finished the drive, total time: %ld:%ld:%ld\n",
			diff_time.tv_sec, diff_time.tv_nsec / 1000000,
			(diff_time.tv_nsec / 1000) % 1000);

	gps_stream(&gps_data, WATCH_DISABLE, NULL);
	gps_close(&gps_data);
}

gboolean drive_line_button_press_event(GtkWidget *widget,
				GdkEventButton *event,
				gpointer user_data)
{
	cmd_args *args = user_data;

	drive_line(*args);
	return false;
}
