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
	int ret;

	// gps_data = connect_to_gpsd(args);

	fd = fopen(args.gpx, "r");

	if (fd == NULL) {
		fprintf(stderr, "Unable to open GPX file %s for reading\n",
			    args.gpx);
		exit(-1);
	}

	// gps_stream(&gps_data, WATCH_ENABLE | WATCH_JSON, NULL);

	/* Find the start and end position from the recorded line */
	/* This is untested and probably unsecure */
	first_line = (char*) malloc(256 * sizeof(char));
	fgets(first_line, 256, fd);

	fprintf(stderr, "%s\n", first_line);

	fseek(fd, 0, SEEK_END);
	last_line = (char*) malloc(256 * sizeof(char));
	fgets(last_line, 256, fd);

	fprintf(stderr, "%s\n", last_line);

	tmp = strtok(first_line, " ");
	while (tmp) {
		if (!strcmp(tmp, "longitude")) {
			start.lon = atoi(strtok(first_line, ","));
		} else if (!strcmp(tmp, "latitude")) {
			start.lat = atoi(strtok(first_line, ","));
		}

		tmp = strtok(NULL, " ");
	}

	tmp = strtok(last_line, " ");
	while (tmp) {
		if (!strcmp(tmp, "longitude")) {
			end.lon = atoi(strtok(first_line, ","));
		} else if (!strcmp(tmp, "latitude")) {
			end.lat = atoi(strtok(first_line, ","));
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

			if (gps_data.fix.latitude == start.lat ||
				gps_data.fix.longitude == start.lat) {
				break;
			}
		} else {
			sleep(1);
		}
	}

	/* Poll until we hit the end line and do stuff */
	while (1) {
		if (gps_waiting(&gps_data, 500)) {
			ret = gps_read(&gps_data);

			if (ret < 0) {
				fprintf(stderr, "gps_read error: %d\n", ret);
				exit(1);
			}

			if (gps_data.fix.latitude == end.lat ||
				gps_data.fix.longitude == end.lat) {
				break;
			}
		} else {
			sleep(1);
		}
	}

	free(first_line);
	free(last_line);
	fclose(fd);
	// gps_stream(&gps_data, WATCH_DISABLE, NULL);
	// gps_close(&gps_data);
}
