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
#include "common.h"
#include "arg-parser.h"
#include "record-track.h"

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

int main(int argc, char **argv)
{
	cmd_args args;

	args.mode = NONE;

	argp_parse(&argp, argc, argv, 0, 0, &args);

	if (args.mode == NONE) {
		fprintf(stderr, "You need to specify a mode\n");
		exit(1);
	}

	/* Do more argument error checking */

	if (args.mode == RECORD_TRACK) {
		record_track(args);
	}

	return 0;
}