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

#ifndef OBDII_COMMANDS_H
#define OBDII_COMMANDS_H

enum command_type {
	OBDII_RPM,
	OBDII_THROTTLE,
	OBDII_ENGINE_LOAD,
	OBDII_TIMING_ADV,
	OBDII_MAF,
	OBDII_COOLANT_TEMP,
	OBDII_INTAKE_TEMP,
	OBDII_SHORT_FUEL_T1,
	OBDII_LONG_FUEL_T1,
	OBDII_FUEL_STATUS
} command_type;

enum return_type {
	RET_LONG,
	RET_FLOAT,
	RET_STR,
	RET_UNICODE
} return_type;

typedef struct obdii_loop_data
{
	gtk_user_data *data;

	PyObject *pModule;
} obdii_loop_data;

typedef struct python_args {
	gtk_user_data *data;
	PyObject *pValue;
	enum command_type com_type;
} python_args;

typedef struct obdii_commands {
	enum command_type com_type;
	char *name;
	enum return_type ret_type;
} obdii_commands;

gpointer obdii_start_connection(gpointer user_data);

#endif /* OBDII_COMMANDS_H */
