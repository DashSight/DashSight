# Copyright 2018: Alistair Francis <alistair@alistair23.me>
#
# See the LICENSE file for license information.
#
# The above copyright notice and this permission notice shall be included in
# all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
# THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
# THE SOFTWARE.

CC ?= gcc
CFLAGS ?= -g
LFLAGS ?= -lm -lgps

GTK_CFLAGS = $(shell pkg-config --cflags gtk+-3.0 osmgpsmap-1.0)
GTK_LFLAGS = $(shell pkg-config --libs gtk+-3.0 osmgpsmap-1.0)

PYTHON_CFLAGS =  $(shell python3-config --includes)
PYTHON_LFALGS =  $(shell python3-config --libs)

.PHONY: all
all: DashSight

OBJECTS = $(patsubst %.c, %.o, $(wildcard *.c)) $(patsubst %.c, %.o, $(wildcard ./*/*.c))
HEADERS = $(wildcard include/*.h)

.PRECIOUS: DashSight $(OBJECTS)

DashSight: $(OBJECTS)
	$(CC) $(GTK_CFLAGS) $(PYTHON_CFLAGS) $(OBJECTS) -Wall $(LFLAGS) -o $@ $(GTK_LFLAGS) $(PYTHON_LFALGS)

%.o: %.c $(HEADERS)
	$(CC) $(CFLAGS) $(GTK_CFLAGS) $(PYTHON_CFLAGS) -Iinclude -c $< -o $@ $(GTK_LFLAGS) $(PYTHON_LFALGS)

.PHONY: clean
clean:
	@rm -f *.o
	@rm -f */*.o
	@rm -f lap-timer
	@rm -f DashSight
