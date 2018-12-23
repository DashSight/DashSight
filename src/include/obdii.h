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
	OBDII_SHORT_O2_T1,
	OBDII_LONG_O2_T1
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
