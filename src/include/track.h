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

#ifndef TRACK_H
#define TRACK_H

#include <gtk/gtk.h>
#include <osm-gps-map.h>
#include "common.h"

#define MAP_ZOOM_LEVEL 15

typedef struct track_info
{
	float lon, lat;
	struct timespec time;
} track_info;

typedef struct track
{
	track_info start, end;
	bool loop;

	OsmGpsMapTrack *osm_track;
} track;

gpointer record_track(gpointer data);
track *load_track(char *file, bool loop);

gboolean record_button_press_event(GtkWidget *widget,
				GdkEventButton *event,
				gpointer user_data);
gboolean drive_line_button_press_event(GtkWidget *widget,
				GdkEventButton *event,
				gpointer user_data);

#endif /* TRACK_H */
