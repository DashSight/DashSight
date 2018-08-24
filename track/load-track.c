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

track load_track(char *file, bool loop)
{
	FILE *fd;
	char *line, *tmp;
	struct timespec cur_time, diff_time;
	bool first_run = true;
	track ret = { 0 };

	fd = fopen(file, "r");

	if (fd == NULL) {
		fprintf(stderr, "Unable to open GPX file %s for reading\n",
			    file);
		exit(-1);
	}

	line = (char*) malloc(256 * sizeof(char));

	do {
		line = fgets(line, 256, fd);

		tmp = strtok(line, " ");
		while (tmp) {
			if (!strcmp(tmp, "longitude:")) {
				if (first_run) {
					ret.start.lon = atof(strtok(NULL, ","));
					ret.end.lon = ret.start.lon;
				} else if (!loop) {
					ret.end.lon = atof(strtok(NULL, ","));
				}
			} else if (!strcmp(tmp, "latitude:")) {
				if (first_run) {
					ret.start.lat = atof(strtok(NULL, ","));
					ret.end.lat = ret.start.lat;
				} else if (!loop) {
					ret.end.lat = atof(strtok(NULL, ","));
				}
			}

			tmp = strtok(NULL, " ");
		}
		first_run = true;
	} while (line);

	free(line);
	fclose(fd);

	ret.loop = loop;

	return ret;
}
