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

#ifndef DRIVE_H
#define DRIVE_H

#include <Python.h>
#include <stdbool.h>
#include <gtk/gtk.h>
#include <gps.h>
#include <osm-gps-map.h>
#include "common.h"
#include "track.h"

typedef struct drive_loop_data
{
	gtk_user_data *data;

	struct gps_data_t gps_data;
	struct timespec *start_time;
	struct timespec best_time;
	OsmGpsMap *map;
	track *cur_track;
} drive_loop_data;

enum gtk_type_enum {
	DRIVE_PROGRESS_BAR,
	DRIVE_LABEL
} gtk_type_enum;

typedef struct drive_display {
	enum drive_disp_type type;
	enum gtk_type_enum gtk_type;
	const char *name;
	const char *zero;
	const char *context_name;
	const char *format;
	int start_x;
	int start_y;
} drive_display;

#define LOCATION_MARGIN 0.00005

#endif /* DRIVE_H; */
