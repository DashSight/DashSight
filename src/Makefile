# Copyright 2018 Alistair Francis <alistair@alistair23.me>
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#    http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

CC ?= gcc
CFLAGS ?= -g
LDFLAGS ?= -lm

GTK_CFLAGS = $(shell pkg-config --cflags gtk+-3.0 osmgpsmap-1.0)
GTK_LDFLAGS = $(shell pkg-config --libs gtk+-3.0 osmgpsmap-1.0)

PYTHON_CFLAGS = $(shell python3-config --includes)
PYTHON_LDFLAGS = $(shell python3-config --libs)

GPSD_LDFLAGS = -lgps

.PHONY: all
all: DashSight

OBJECTS = $(patsubst %.c, %.o, $(wildcard *.c)) $(patsubst %.c, %.o, $(wildcard ./*/*.c))
HEADERS = $(wildcard include/*.h)

.PRECIOUS: DashSight $(OBJECTS)

DashSight: $(OBJECTS)
	$(CC) $(GTK_CFLAGS) $(PYTHON_CFLAGS) $(OBJECTS) -Wall $(LDFLAGS) -o $@ $(GTK_LDFLAGS) $(PYTHON_LDFLAGS) $(GPSD_LDFLAGS)

%.o: %.c $(HEADERS)
	$(CC) $(CFLAGS) $(GTK_CFLAGS) $(PYTHON_CFLAGS) -Iinclude -c $< -o $@ $(GTK_LDFLAGS) $(PYTHON_LDFLAGS) $(GPSD_LDFLAGS)

.PHONY: clean
clean:
	@rm -f *.o
	@rm -f */*.o
	@rm -f lap-timer
	@rm -f DashSight
