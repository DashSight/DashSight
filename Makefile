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

.PHONY: all
all: lap-timer

OBJECTS = $(patsubst %.c, %.o, $(wildcard *.c))
HEADERS = $(wildcard include/*.h)

.PRECIOUS: lap-timer $(OBJECTS)

lap-timer: $(OBJECTS)
	$(CC) $(OBJECTS) -Wall $(LFLAGS) -o $@

%.o: %.c $(HEADERS)
	$(CC) $(CFLAGS) -Iinclude -c $< -o $@

.PHONY: clean
clean:
	@rm -f *.o
	@rm -f lap-timer