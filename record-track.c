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
#include "record-track.h"

void record_track(cmd_args args)
{
	FILE *fd;
	struct gps_data_t gps_data;
	int ret;

	gps_data = connect_to_gpsd(args);

	fd = fopen(args.gpx, "r");

	if (fd == NULL) {
		fprintf(stderr, "Unable to open GPX file %s for reading\n",
			    args.gpx);
		exit(-1);
	}

	gps_stream(&gps_data, WATCH_ENABLE | WATCH_JSON, NULL);

	/* Read data and write to file until user interrupts us */
	while (1) {
		if (gps_waiting(&gps_data, 500)) {
			ret = gps_read(&gps_data);

			if (ret < 0) {
				fprintf(stderr, "gps_read error: %d\n", ret);
				exit(1);
			}

			if (gps_data.set) {
				/* Fix this to be in a real formart */
				fprintf(fd, "mode: %d, ", gps_data.fix.mode);
				fprintf(fd, "time: %10.0f, ", gps_data.fix.time);
				fprintf(fd, "latitude: %f, ", gps_data.fix.latitude);
				fprintf(fd, "longitude: %f, ", gps_data.fix.longitude);
				fprintf(fd, "altitude: %f, ", gps_data.fix.altitude);
				fprintf(fd, "speed: %f, ", gps_data.fix.speed);
				fprintf(fd, "track: %f, ", gps_data.fix.track);
				fprintf(fd, "pdop: %f", gps_data.dop.pdop);
				fprintf(fd, "\n");
			}
		} else {
			sleep(1);
		}
	}

	fclose(fd);
	gps_stream(&gps_data, WATCH_DISABLE, NULL);
	gps_close(&gps_data);
}
