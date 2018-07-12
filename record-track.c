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
#include <math.h>
#include <gps.h>
#include "common.h"
#include "record-track.h"

void record_track(cmd_args args)
{
	FILE *fd;
	struct gps_data_t gps_data;
	struct timeval tv;
	int ret;

	gps_data = connect_to_gpsd(args);

	fd = fopen(args.gpx, "r");

	if (fd == NULL) {
		fprintf(stderr, "Unable to open GPX file %s for reading\n",
			    args.gpx);
		exit(-1);
	}

	/* Read data and write to file until user interrupts us */
	while (1) {
		ret = gps_read(&gps_data);

		if (ret < 0) {
			fprintf(stderr, "gps_read error: %d\n", ret);
			exit(1);
		}

		if ((gps_data.status == STATUS_FIX) &&
			(gps_data.fix.mode == MODE_2D || gps_data.fix.mode == MODE_3D) &&
			!isnan(gps_data.fix.latitude) &&
			!isnan(gps_data.fix.longitude)) {
			gettimeofday(&tv, NULL);
			printf("height: %f, latitude: %f, longitude: %f, speed: %f, timestamp: %f\n", gps_data.fix.altitude, gps_data.fix.latitude, gps_data.fix.longitude, gps_data.fix.speed, gps_data.fix.time/*tv.tv_sec*/);
		}
	}

	gps_close(&gps_data);
}
