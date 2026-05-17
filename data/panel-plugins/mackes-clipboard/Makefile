# Makefile — build the Mackes mesh-clipboard xfce4-panel external plugin.
#
# Standalone build (no autotools / no meson). Uses pkg-config to discover
# GTK3 + xfce4-panel cflags and libs. RPM build runs `make` here under
# the %build section.

CC          ?= gcc
PKG_CONFIG  ?= pkg-config
CFLAGS      ?= -O2 -g -Wall -Wextra -Wno-unused-parameter
LDFLAGS     ?=

PKGS        = gtk+-3.0 libxfce4panel-2.0 libxfce4util-1.0 gdk-x11-3.0
PKG_CFLAGS  := $(shell $(PKG_CONFIG) --cflags $(PKGS) 2>/dev/null)
PKG_LIBS    := $(shell $(PKG_CONFIG) --libs   $(PKGS) 2>/dev/null)

BIN         = mackes-clipboard
SRC         = mackes-clipboard.c

prefix      ?= /usr
libdir      ?= $(prefix)/lib
datadir     ?= $(prefix)/share

PLUGIN_LIBDIR    = $(libdir)/xfce4/panel/plugins
PLUGIN_DESKTOPDIR = $(datadir)/xfce4/panel/plugins

all: $(BIN)

$(BIN): $(SRC)
	$(CC) $(CFLAGS) $(PKG_CFLAGS) $(SRC) -o $(BIN) $(LDFLAGS) $(PKG_LIBS)

install: $(BIN)
	install -D -m 0755 $(BIN) $(DESTDIR)$(PLUGIN_LIBDIR)/$(BIN)
	install -D -m 0644 $(BIN).desktop \
	    $(DESTDIR)$(PLUGIN_DESKTOPDIR)/$(BIN).desktop

uninstall:
	rm -f $(DESTDIR)$(PLUGIN_LIBDIR)/$(BIN)
	rm -f $(DESTDIR)$(PLUGIN_DESKTOPDIR)/$(BIN).desktop

clean:
	rm -f $(BIN) *.o

check-deps:
	@$(PKG_CONFIG) --print-errors --exists $(PKGS) && \
	    echo "all pkg-config deps satisfied" || \
	    { echo "missing pkg-config dep — install xfce4-panel-devel + libxfce4util-devel + gtk3-devel"; exit 1; }

.PHONY: all install uninstall clean check-deps
