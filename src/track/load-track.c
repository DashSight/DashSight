/*
 * Copyright 2018 Alistair Francis <alistair@alistair23.me>
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *    http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <math.h>
#include <gps.h>
#include "common.h"
#include "track.h"

track *load_track(char *file, bool loop)
{
	FILE *fd;
	char *line, *tmp, *tmp_dup;
	struct timespec cur_time, diff_time;
	OsmGpsMapPoint *point;
	bool first_run = true;
	track *ret = g_new0(track, 1);
	float cur_lat, cur_lon;

	fd = fopen(file, "r");

	if (fd == NULL) {
		fprintf(stderr, "Unable to open GPX file %s for reading\n",
			    file);
		exit(-1);
	}

	ret->osm_track = osm_gps_map_track_new();

	line = (char*) malloc(256 * sizeof(char));
	line = fgets(line, 256, fd);

	/* TODO: Check the meta data */

	/* Skip the XML data and look for the track starting */
	while (line) {
		tmp = strtok(line, " ");
		tmp[strcspn(tmp, "\r\n")] = 0;
		if (tmp && !strcmp(tmp, "<trk>")) {
			break;
		}

		line = fgets(line, 256, fd);
	}

	line = fgets(line, 256, fd);

	while (line) {
		line[strcspn(line, "\r\n")] = 0;

		tmp = g_strrstr(line, "lat=\"");
		if (tmp) {
			tmp_dup = g_strdup(tmp);
			tmp = strtok(tmp_dup, "\"");
			tmp = strtok(NULL, "\"");
			cur_lat = atof(tmp);
			if (first_run) {
				ret->start.lat = cur_lat;
				ret->end.lat = cur_lat;
			} else if (!loop) {
				ret->end.lat = cur_lat;
			}
			g_free(tmp_dup);
		}

		tmp = g_strrstr(line, "lon=\"");
		if (tmp) {
			tmp_dup = g_strdup(tmp);
			tmp = strtok(tmp_dup, "\"");
			tmp = strtok(NULL, "\"");
			cur_lon = atof(tmp);
			if (first_run) {
				ret->start.lon = cur_lon;
				ret->end.lon = cur_lon;
			} else if (!loop) {
				ret->end.lon = cur_lon;
			}
			g_free(tmp_dup);

			point = osm_gps_map_point_new_degrees(cur_lat, cur_lon);
			osm_gps_map_track_add_point(ret->osm_track, point);
			osm_gps_map_point_free(point);
			first_run = false;
		}

		line = fgets(line, 256, fd);
	}

	free(line);
	fclose(fd);

	ret->loop = loop;

	return ret;
}
