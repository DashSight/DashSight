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

enum ret_type {
	RET_LONG,
	RET_FLOAT,
	RET_STR,
	RET_UNICODE
} ret_type;

typedef struct obdii_commands {
	char *name;
	enum ret_type return_type;
} obdii_commands;

long python_parse_long(PyObject *pValue);
float python_parse_float(PyObject *pValue);
char *python_parse_unicode(PyObject *pValue);
char *python_parse_str(PyObject *pValue);

#endif /* OBDII_COMMANDS_H */
