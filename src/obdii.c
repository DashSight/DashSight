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
	char *temp, *format, *markup;

	if (PyLong_Check(pValue)) {
		ret = PyLong_AsLong(pValue);
	}

	switch (com_type) {
	case OBDII_COOLANT_TEMP:
		format = COOLANT_FORMAT;
		temp = g_strdup_printf("%lu", ret);
		markup = g_markup_printf_escaped(format, temp);

		g_mutex_lock(&(data->draw_update));
		gtk_label_set_markup(GTK_LABEL(data->coolant_temp_disp), markup);
		g_mutex_unlock(&(data->draw_update));
		g_free(temp);
		g_free(markup);
		break;
	case OBDII_INTAKE_TEMP:
		format = INTAKE_FORMAT;
		temp = g_strdup_printf("%lu", ret);
		markup = g_markup_printf_escaped(format, temp);

		g_mutex_lock(&(data->draw_update));
		gtk_label_set_markup(GTK_LABEL(data->intake_temp_disp), markup);
		g_mutex_unlock(&(data->draw_update));
		g_free(temp);
		g_free(markup);
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
		g_mutex_lock(&(data->draw_update));
		gtk_widget_queue_draw(data->taco_draw_area);
		g_mutex_unlock(&(data->draw_update));
		break;
	case OBDII_THROTTLE:
		g_mutex_lock(&(data->draw_update));
		gtk_progress_bar_set_fraction(GTK_PROGRESS_BAR(data->throttle_bar),
									ret / 100.0);
		g_mutex_unlock(&(data->draw_update));
		break;
	case OBDII_ENGINE_LOAD:
		g_mutex_lock(&(data->draw_update));
		gtk_progress_bar_set_fraction(GTK_PROGRESS_BAR(data->engine_load_bar),
									ret / 100.0);
		g_mutex_unlock(&(data->draw_update));
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

gboolean obdii_loop(gpointer user_data)
{
	obdii_loop_data *obdii_data = user_data;
	gtk_user_data *data = obdii_data->data;
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
			switch (obdii_sur_coms[i].ret_type) {
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

	source = g_timeout_source_new(175);
	g_source_set_callback(source, obdii_loop, obdii_data, NULL);
	pid = g_source_attach(source, worker_context);

	g_main_loop_run(data->obdii_loop);
	g_main_loop_unref(data->obdii_loop);

	g_source_remove(pid);
	g_free(obdii_data);

	Py_DECREF(pModule);
	Py_Finalize();

	g_object_unref(data->drive_container);

	g_main_context_pop_thread_default(worker_context);
	g_main_context_unref(worker_context);

	return NULL;
}
