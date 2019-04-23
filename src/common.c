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
#include <gps.h>
#include <math.h>
#include <gtk/gtk.h>
#include "common.h"
#include "track.h"

bool equal(float a, float b, float epsilon)
{
	return fabs(a - b) < epsilon;
}

struct timespec timeval_subtract(struct timespec *x, struct timespec *y)
{
	struct timespec result;
	int nsec;

	result.tv_sec = x->tv_sec - y->tv_sec;

	if ((result.tv_nsec = x->tv_nsec - y->tv_nsec) < 0) {
		result.tv_nsec += 1000000000;
		result.tv_sec--;
	}

	return result;
}

/* Returns true if x > y */
bool timeval_cmp(struct timespec *x, struct timespec *y)
{
	if (x->tv_sec > y->tv_sec) {
		return true;
	}

	if (x->tv_sec == y->tv_sec) {
		if (x->tv_nsec > y->tv_nsec) {
			return true;
		}
	}

	return false;
}

struct gps_data_t connect_to_gpsd(cmd_args args)
{
	struct gps_data_t gps_data;
	int err;

	err = gps_open(args.server, args.port, &gps_data);

	/* Do error checking */
	if (err) {
		fprintf(stderr, "Failed to conncet to %s:%s, error: %d\n",
			    args.server, args.port, err);
		exit(-1);
	}

	/* This needs to be closed with gps_close() */
	return gps_data;
}
