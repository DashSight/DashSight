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
