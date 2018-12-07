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

#include <Python.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <math.h>
#include <gps.h>
#include "track.h"
#include "common.h"
#include "obdii_commands.h"

obdii_commands obdii_sur_coms[] = {
	{ OBDII_RPM,          "RPM",             RET_FLOAT },
	{ OBDII_THROTTLE,     "THROTTLE_POS",    RET_FLOAT },
	{ OBDII_ENGINE_LOAD,  "ENGINE_LOAD",     RET_FLOAT },
	{ OBDII_TIMING_ADV,   "TIMING_ADVANCE",  RET_FLOAT },
	{ OBDII_MAF,          "MAF",             RET_FLOAT },
	{ OBDII_COOLANT_TEMP, "COOLANT_TEMP",    RET_LONG },
	{ OBDII_INTAKE_TEMP,  "INTAKE_TEMP",     RET_LONG },
};

long python_parse_long(gtk_user_data *data,
						PyObject *pValue,
						enum command_type com_type) {
	long ret = 0;
	const char *format = COOLANT_FORMAT;
	char *temp;
	char *markup;

	if (PyLong_Check(pValue)) {
		ret = PyLong_AsLong(pValue);
	}

	switch (com_type) {
	case OBDII_COOLANT_TEMP:
		temp = g_strdup_printf("%l *C", ret);
		markup = g_markup_printf_escaped(format, temp);
		gtk_label_set_markup(GTK_LABEL(data->coolant_temp_disp), markup);
		g_free(temp);
		g_free(markup);
		break;
	case OBDII_INTAKE_TEMP:
		/* Display air intake temp */
		break;
	}

	return ret;
}

float python_parse_float(gtk_user_data *data,
						PyObject *pValue,
						enum command_type com_type) {
	float ret = 0;

	if (PyFloat_Check(pValue)) {
		ret = PyFloat_AsDouble(pValue);
	}

	switch (com_type) {
	case OBDII_RPM:
		data->revs = ret;
		gtk_widget_queue_draw(data->taco_draw_area);
		break;
	case OBDII_THROTTLE:
		gtk_progress_bar_set_fraction(GTK_PROGRESS_BAR(data->throttle_bar),
									ret / 100.0);
		break;
	case OBDII_ENGINE_LOAD:
		/* Display engine load */
		break;
	case OBDII_TIMING_ADV:
		/* Display timing advance info */
		break;
	case OBDII_MAF:
		/* Display the MAF */
		break;
	}

	return ret;
}

char *python_parse_unicode(PyObject *pValue) {
	 if (PyBytes_Check(pValue)) {
		fprintf(stderr, "B: %s\n", PyBytes_AsString(pValue));
	}

	return PyBytes_AsString(pValue);
}

char *python_parse_str(PyObject *pValue) {
	if (PyUnicode_Check(pValue)) {
		fprintf(stderr, "U: %s\n", PyUnicode_AsUTF8(pValue));
	}

	return PyUnicode_AsUTF8(pValue);
}

gpointer obdii_data(gpointer user_data)
{
	gtk_user_data *data = user_data;
	PyObject *pName, *pModule;
	PyObject *pFunc, *pValue;
	PyObject *pArg0, *pArgs;
	int i;

	Py_Initialize();

	pName = PyUnicode_DecodeFSDefault("obdii_connect");
	pModule = PyImport_Import(pName);
	Py_DECREF(pName);

	if (!pModule) {
		fprintf(stderr, "Unable to import Python module\n");
		PyErr_Print();
		return NULL;
	}

	/* Don't start updating the page until we have it. */
	while (data->load_page) {
		sleep(1);
	}

	while (true) {
		for (i = 0; i < ARRAY_SIZE(obdii_sur_coms); i++) {
			pArgs = PyTuple_New(1);
			pArg0 = PyUnicode_FromString(obdii_sur_coms[i].name);

			if (!pArg0) {
				Py_DECREF(pArg0);
				Py_DECREF(pModule);
				PyErr_Print();
				fprintf(stderr, "Cannot convert argument\n");
				return NULL;
			}

			PyTuple_SetItem(pArgs, 0, pArg0);
			pFunc = PyObject_GetAttrString(pModule, "c_get_data");

			if (pFunc && PyCallable_Check(pFunc)) {
				pValue = PyObject_CallObject(pFunc, pArgs);
				Py_DECREF(pArgs);

				if (pValue != NULL) {
					switch (obdii_sur_coms->ret_type) {
					case RET_LONG:
						python_parse_long(data, pValue,
											obdii_sur_coms[i].com_type);
						break;
					case RET_FLOAT:
						python_parse_float(data, pValue,
											obdii_sur_coms[i].com_type);
						break;
					case RET_STR:
						python_parse_str(pValue);
						break;
					case RET_UNICODE:
						python_parse_unicode(pValue);
						break;
					}
					Py_DECREF(pValue);
				} else {
					PyErr_Print();
					break;
				}
			}

			Py_DECREF(pFunc);
			usleep(1000);
		}

		sleep(1);
	}

	Py_DECREF(pModule);

	Py_Finalize();

	return NULL;
}
