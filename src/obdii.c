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

long python_parse_long(PyObject *pValue) {
	if (PyLong_Check(pValue)) {
		fprintf(stderr, "L: %ld\n", PyLong_AsLong(pValue));
	}

	return PyLong_AsLong(pValue);
}

float python_parse_float(gtk_user_data *data, PyObject *pValue, enum command_type com_type) {
	float ret;

	if (PyFloat_Check(pValue)) {
		ret = PyFloat_AsDouble(pValue);
	}

	switch (com_type) {
	case OBDII_RPM:
		/* Update the RPM widget */
		break;
	case OBDII_THROTTLE:
		/* Update the throttle widget */
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