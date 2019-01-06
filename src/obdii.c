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
#include "obdii.h"

obdii_commands obdii_sur_coms[] = {
	{ OBDII_RPM,          "RPM",                RET_FLOAT },
	{ OBDII_THROTTLE,     "THROTTLE_POS",       RET_FLOAT },
	{ OBDII_ENGINE_LOAD,  "ENGINE_LOAD",        RET_FLOAT },
	{ OBDII_TIMING_ADV,   "TIMING_ADVANCE",     RET_FLOAT },
	{ OBDII_MAF,          "MAF",                RET_FLOAT },
	{ OBDII_COOLANT_TEMP, "COOLANT_TEMP",       RET_LONG  },
	{ OBDII_INTAKE_TEMP,  "INTAKE_TEMP",        RET_LONG  },
	{ OBDII_SHORT_O2_T1,  "SHORT_O2_TRIM_B1",   RET_LONG  },
	{ OBDII_LONG_O2_T1,   "LONG_O2_TRIM_B1",    RET_LONG  },
	{ OBDII_FUEL_STATUS,  "FUEL_STATUS",        RET_STR   },
};

static gboolean python_parse_long(gpointer python_data)
{
	python_args *args = python_data;
	gtk_user_data *data = args->data;
	PyObject *pValue = args->pValue;
	enum command_type com_type = args->com_type;
	long ret = 0;
	char *temp, *format, *markup;

	g_assert(g_main_context_get_thread_default() == g_main_context_default() ||
			g_main_context_get_thread_default() == NULL);

	if (PyLong_Check(pValue)) {
		ret = PyLong_AsLong(pValue);
	}

	switch (com_type) {
	case OBDII_COOLANT_TEMP:
		format = COOLANT_FORMAT;
		temp = g_strdup_printf("%lu", ret);
		markup = g_markup_printf_escaped(format, temp);

		gtk_label_set_markup(GTK_LABEL(data->ddisp_widgets[COOLANT_TEMP]), markup);
		g_free(temp);
		g_free(markup);
		break;
	case OBDII_INTAKE_TEMP:
		format = INTAKE_FORMAT;
		temp = g_strdup_printf("%lu", ret);
		markup = g_markup_printf_escaped(format, temp);

		gtk_label_set_markup(GTK_LABEL(data->ddisp_widgets[INTAKE_TEMP]), markup);
		g_free(temp);
		g_free(markup);
		break;
	case OBDII_SHORT_O2_T1:
		format = SHORT_O2_T1_FORMAT;
		temp = g_strdup_printf("%lu", ret);
		markup = g_markup_printf_escaped(format, temp);

		gtk_label_set_markup(GTK_LABEL(data->ddisp_widgets[SHORT_O2_B1]), markup);
		g_free(temp);
		g_free(markup);
		break;
	case OBDII_LONG_O2_T1:
		format = LONG_O2_T1_FORMAT;
		temp = g_strdup_printf("%lu", ret);
		markup = g_markup_printf_escaped(format, temp);

		gtk_label_set_markup(GTK_LABEL(data->ddisp_widgets[LONG_O2_B1]), markup);
		g_free(temp);
		g_free(markup);
		break;
	}

	return false;
}

static gboolean python_parse_float(gpointer python_data)
{
	python_args *args = python_data;
	gtk_user_data *data = args->data;
	PyObject *pValue = args->pValue;
	enum command_type com_type = args->com_type;
	float ret = 0;
	char *temp, *format, *markup;

	g_assert(g_main_context_get_thread_default() == g_main_context_default() ||
			g_main_context_get_thread_default() == NULL);

	if (PyFloat_Check(pValue)) {
		ret = PyFloat_AsDouble(pValue);
	}

	switch (com_type) {
	case OBDII_RPM:
		data->revs = ret;
		gtk_widget_queue_draw(data->taco_draw_area);
		break;
	case OBDII_THROTTLE:
		gtk_progress_bar_set_fraction(GTK_PROGRESS_BAR(data->ddisp_widgets[THROTTLE_BAR]),
									ret / 100.0);
		break;
	case OBDII_ENGINE_LOAD:
		gtk_progress_bar_set_fraction(GTK_PROGRESS_BAR(data->ddisp_widgets[LOAD_BAR]),
									ret / 100.0);
		break;
	case OBDII_TIMING_ADV:
		format = MAF_FORMAT;
		temp = g_strdup_printf("%.0f", ret);
		markup = g_markup_printf_escaped(format, temp);

		gtk_label_set_markup(GTK_LABEL(data->ddisp_widgets[TIMING_ADVANCED]), markup);
		g_free(temp);
		g_free(markup);
		break;
	case OBDII_MAF:
		format = MAF_FORMAT;
		temp = g_strdup_printf("%.0f", ret);
		markup = g_markup_printf_escaped(format, temp);

		gtk_label_set_markup(GTK_LABEL(data->ddisp_widgets[MAF]), markup);
		g_free(temp);
		g_free(markup);
		break;
	}

	return false;
}

static gboolean python_parse_str(gpointer python_data)
{
	python_args *args = python_data;
	gtk_user_data *data = args->data;
	PyObject *pValue = args->pValue;
	enum command_type com_type = args->com_type;
	char *ret;
	char *temp, *format, *markup;

	g_assert(g_main_context_get_thread_default() == g_main_context_default() ||
			g_main_context_get_thread_default() == NULL);

	if (PyUnicode_Check(pValue)) {
		ret = (char*) PyUnicode_AsUTF8(pValue);
	}

	switch (com_type) {
	case OBDII_FUEL_STATUS:
		format = FUEL_STATUS_FORMAT;
		temp = g_strdup_printf("%s", ret);
		markup = g_markup_printf_escaped(format, temp);

		gtk_label_set_markup(GTK_LABEL(data->ddisp_widgets[FUEL_STATUS]), markup);
		g_free(temp);
		g_free(markup);
	}

	return false;
}

static char *python_parse_unicode(PyObject *pValue) {
	 if (PyBytes_Check(pValue)) {
		fprintf(stderr, "B: %s\n", PyBytes_AsString(pValue));
	}

	return PyBytes_AsString(pValue);
}

static void python_parse_notify_free(gpointer data)
{
	python_args *args = data;

	Py_DECREF(args->pValue);
	g_free(data);
}

gboolean obdii_loop(gpointer user_data)
{
	obdii_loop_data *obdii_data = user_data;
	gtk_user_data *data = obdii_data->data;
	python_args *args = g_new0(python_args, 1);
	PyObject *pFunc, *pValue;
	PyObject *pArgs;
	PyObject *pModule = obdii_data->pModule;
	static int i = 0;

	if (!data || data->finished_drive) {
		g_main_loop_quit(data->obdii_loop);
		return false;
	}

	pArgs = Py_BuildValue("(s)", obdii_sur_coms[i].name);

	if (!pArgs) {
		Py_DECREF(pArgs);
		Py_DECREF(pModule);
		PyErr_Print();
		fprintf(stderr, "Cannot convert argument\n");
		g_main_loop_quit(data->obdii_loop);
		return false;
	}

	pFunc = PyObject_GetAttrString(pModule, "c_get_data");

	if (pFunc && PyCallable_Check(pFunc)) {
		pValue = PyObject_CallObject(pFunc, pArgs);

		if (pValue != NULL) {
			if (PyLong_Check(pValue) && PyLong_AsLong(pValue) == -1) {
				/* There was an error form the OBD device */
				Py_DECREF(pValue);
				Py_DECREF(pArgs);
				Py_DECREF(pFunc);
				g_main_loop_quit(data->obdii_loop);
				return false;
			}

			args->data = data;
			args->pValue = pValue;
			args->com_type = obdii_sur_coms[i].com_type;

			switch (obdii_sur_coms[i].ret_type) {
			case RET_LONG:
				g_main_context_invoke_full(g_main_context_default(),
											G_PRIORITY_DEFAULT,
											python_parse_long, args,
											python_parse_notify_free);
				break;
			case RET_FLOAT:
				g_main_context_invoke_full(g_main_context_default(),
											G_PRIORITY_DEFAULT,
											python_parse_float, args,
											python_parse_notify_free);
				break;
			case RET_STR:
				g_main_context_invoke_full(g_main_context_default(),
											G_PRIORITY_DEFAULT,
											python_parse_str, args,
											python_parse_notify_free);
				break;
			case RET_UNICODE:
				python_parse_unicode(pValue);
				break;
			}
		} else {
			PyErr_Print();
			g_main_loop_quit(data->obdii_loop);
			return false;
		}
	}

	Py_DECREF(pArgs);
	Py_DECREF(pFunc);

	i++;

	if (i >= ARRAY_SIZE(obdii_sur_coms)) {
		i = 0;
	}

	return true;
}

gpointer obdii_start_connection(gpointer user_data)
{
	gtk_user_data *data = user_data;
	PyObject *pName, *pModule;
	GMainContext *worker_context;
	GSource *source;
	int pid;

	worker_context = g_main_context_new();
	g_main_context_push_thread_default(worker_context);

	while (!data->finished_drive) {
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
		while (data->load_page && !data->finished_drive) {
			sleep(1);
		}

		g_object_ref(data->drive_container);

		obdii_loop_data *obdii_data = g_new0(obdii_loop_data, 1);;
		obdii_data->data = data;
		obdii_data->pModule = pModule;

		data->obdii_loop = g_main_loop_new(worker_context, false);

		source = g_timeout_source_new(125);
		g_source_set_callback(source, obdii_loop, obdii_data, NULL);
		pid = g_source_attach(source, worker_context);

		g_main_context_unref(worker_context);
		g_source_unref(source);

		g_main_loop_run(data->obdii_loop);
		g_main_loop_unref(data->obdii_loop);

		g_free(obdii_data);

		Py_DECREF(pModule);
		Py_Finalize();

		if (!data->finished_drive) {
			/* We are going to loop again, sleep for a little bit
			 * as we probably don't have a connection.
			 */
			sleep(5);
		}
	}

	g_object_unref(data->drive_container);

	g_main_context_pop_thread_default(worker_context);

	return NULL;
}
