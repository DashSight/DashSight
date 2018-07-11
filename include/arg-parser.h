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

struct arguments {
	enum { NONE, RECORD_TRACK, CIRC_DRIVE, SINGLE_DRIVE } mode;
};

static struct argp_option options[] = { 
	{ "record-track", 'r', 0, 0, "Record a new track"},
	{ "circle-drive", 'c', 0, 0, "Drive a track"},
	{ "single-drive", 's', 0, 0, "Drive a single track"},
	{ 0 } 
};

static error_t parse_opt(int key, char *arg, struct argp_state *state) {

	struct arguments *arguments = state->input;

	switch (key) {
	case 'r':
		arguments->mode = RECORD_TRACK;
		break;
	case 'c':
		arguments->mode = CIRC_DRIVE;
		break;
	case 's':
		arguments->mode = SINGLE_DRIVE;
		break;
	default: return ARGP_ERR_UNKNOWN;
	}
	return 0;
}

static struct argp argp = { options, parse_opt, 0, 0, 0, 0, 0 };

#endif /* ARG_PARSER_H */
