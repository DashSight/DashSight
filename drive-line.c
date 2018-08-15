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
	track_info start, end;
	char *first_line, *last_line;
	char *tmp;
	struct timespec cur_time, diff_time;
	int ret;

	gps_data = connect_to_gpsd(args);

	fd = fopen(args.gpx, "r");

	if (fd == NULL) {
		fprintf(stderr, "Unable to open GPX file %s for reading\n",
			    args.gpx);
		exit(-1);
	}

	gps_stream(&gps_data, WATCH_ENABLE | WATCH_JSON, NULL);

	/* Find the start and end position from the recorded line */
	/* This is untested and probably unsecure */
	first_line = (char*) malloc(256 * sizeof(char));
	fgets(first_line, 256, fd);

	fseek(fd, 0, SEEK_END);
	last_line = (char*) malloc(256 * sizeof(char));
	fgets(last_line, 256, fd);

	tmp = strtok(first_line, " ");
	while (tmp) {
		if (!strcmp(tmp, "longitude:")) {
			start.lon = atof(strtok(NULL, ","));
		} else if (!strcmp(tmp, "latitude:")) {
			start.lat = atof(strtok(NULL, ","));
		}

		tmp = strtok(NULL, " ");
	}

	tmp = strtok(last_line, " ");
	while (tmp) {
		if (!strcmp(tmp, "longitude:")) {
			end.lon = atof(strtok(NULL, ","));
		} else if (!strcmp(tmp, "latitude:")) {
			end.lat = atof(strtok(NULL, ","));
		}

		tmp = strtok(NULL, " ");
	}

	/* Poll until we hit the start line */
	while (1) {
		if (gps_waiting(&gps_data, 500)) {
			ret = gps_read(&gps_data);

			if (ret < 0) {
				fprintf(stderr, "gps_read error: %d\n", ret);
				exit(1);
			}

			if (equal(gps_data.fix.latitude, start.lat, 0.05) ||
				equal(gps_data.fix.longitude, start.lat, 0.05)) {
				clock_gettime(CLOCK_MONOTONIC_RAW, &start.time);
				break;
			}
		} else {
			sleep(1);
		}
	}

	fprintf(stderr, "Starting the drive\n");

	/* Poll until we hit the end line and do stuff */
	while (1) {
		clock_gettime(CLOCK_MONOTONIC_RAW, &cur_time);
		diff_time = timeval_subtract(&cur_time, &start.time);
		printf("Time: %ld - %ld\r", diff_time.tv_sec, diff_time.tv_nsec);
		fflush(stdout);
		if (gps_waiting(&gps_data, 10)) {
			ret = gps_read(&gps_data);

			if (ret < 0) {
				fprintf(stderr, "gps_read error: %d\n", ret);
				exit(1);
			}

			if (equal(gps_data.fix.latitude, end.lat, 0.05) ||
				equal(gps_data.fix.longitude, end.lat, 0.05)) {
				clock_gettime(CLOCK_MONOTONIC_RAW, &end.time);
				break;
			}
		} else {
			sleep(1);
		}
	}

	fprintf(stderr, "Finished the drive\n");

	free(first_line);
	free(last_line);
	fclose(fd);
	gps_stream(&gps_data, WATCH_DISABLE, NULL);
	gps_close(&gps_data);
}
