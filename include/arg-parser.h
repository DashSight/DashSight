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

#ifndef ARG_PARSER_H
#define ARG_PARSER_H

#include <argp.h>
#include "common.h"

static struct argp_option options[] = { 
	{ "record-track", 'r', 0, 0, "Record a new track"},
	{ "circle-drive", 'c', 0, 0, "Drive a track"},
	{ "single-drive", '1', 0, 0, "Drive a single track"},
	{ "gpsd-server",  's', "server", 0, "Specify the GPSd server"},
	{ "gpsd-port",    'p', "port", 0, "Specify the GPSd port"},
	{ "track-gpx",    't', "file", 0, "File to read/write the track GPX data from/to"},
	{ 0 } 
};

static error_t parse_opt(int key, char *arg, struct argp_state *state) {

	cmd_args *arguments = state->input;

	switch (key) {
	case 'r':
		arguments->mode = RECORD_TRACK;
		break;
	case 'c':
		arguments->mode = CIRC_DRIVE;
		break;
	case '1':
		arguments->mode = SINGLE_DRIVE;
		break;
	case 's':
		arguments->server = arg;
		break;
	case 'p':
		arguments->port = arg;
		break;
	case 't':
		arguments->gpx = arg;
		break;
	default:
		return ARGP_ERR_UNKNOWN;
	}
	return 0;
}

static struct argp argp = { options, parse_opt, 0, 0, 0, 0, 0 };

#endif /* ARG_PARSER_H */
