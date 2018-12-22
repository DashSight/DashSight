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

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <gtk/gtk.h>
#include "common.h"

gboolean taco_draw_callback(GtkWidget *widget, cairo_t *cr, gpointer user_data)
{
	gtk_user_data *data = user_data;
	GtkStyleContext *context;
	guint width,height;
	double mid_x = 70;
	double mid_y = 70;
	double radius = 65;
	gchar *revs;
	int i;

	context = gtk_widget_get_style_context(widget);
	width = gtk_widget_get_allocated_width(widget);
	height = gtk_widget_get_allocated_height(widget);

	/* Draw the outside */
	cairo_set_line_width(cr, 1.0);
	cairo_arc(cr, mid_x, mid_y, radius, 0, 2 * M_PI);
	cairo_stroke(cr);

	/* Draw the pointer */
	cairo_set_source_rgba(cr, 1, 0.2, 0.2, 0.6);
	cairo_set_line_width(cr, 6.0);

	cairo_arc(cr, mid_x, mid_y, 10.0, 0, 2 * M_PI);
	cairo_fill(cr);

	cairo_arc(cr, mid_x, mid_y, radius, REV_ANGLE(data->revs), REV_ANGLE(data->revs));
	cairo_line_to(cr, mid_x, mid_y);
	cairo_stroke(cr);

	/* Draw the numbers */
	cairo_set_source_rgba(cr, 0.0, 0.0, 0.0, 1.0);
	cairo_select_font_face(cr, "Sans", CAIRO_FONT_SLANT_NORMAL,
							CAIRO_FONT_WEIGHT_BOLD);
	cairo_set_font_size (cr, 15.0);

	cairo_set_line_width(cr, 0.0);
	for (i = 0; i < 10; i++) {
		cairo_arc(cr, mid_x, mid_y, radius + (11 - i), 0, REV_ANGLE(i * 1000));
		revs = g_strdup_printf("%d", i);
		cairo_show_text(cr, revs);
		g_free(revs);
		cairo_stroke(cr);
	}

	cairo_select_font_face(cr, "Sans", CAIRO_FONT_SLANT_NORMAL,
							CAIRO_FONT_WEIGHT_NORMAL);
	cairo_set_font_size (cr, 10.0);
	cairo_arc(cr, mid_x, mid_y, radius / 2, 0, M_PI * (2.2 / 3.0));
	cairo_show_text(cr, "revs x1000");
	cairo_stroke(cr);

	return false;
}
