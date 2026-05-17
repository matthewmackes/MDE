/*
 * mackes-clipboard — distributed clipboard xfce4-panel plugin (external).
 *
 * Reads the local user's mesh-sync clipboard bucket
 * (~/QNM-Shared/.qnm-sync/clipboard/) for every-peer clipboard items,
 * displays them as a popup menu when the panel button is clicked, and
 * copies the chosen item back to the X11 clipboard. Watches the X11
 * clipboard for local copies and writes them into the mesh-sync bucket.
 *
 * Compiled to /usr/lib/xfce4/panel/plugins/mackes-clipboard.
 * Registered via /usr/share/xfce4/panel/plugins/mackes-clipboard.desktop.
 *
 * GPL-3.0, ©2026 Matthew Mackes.
 */

#include <gtk/gtk.h>
#include <gtk/gtkx.h>          /* GtkPlug — split out of gtk/gtk.h in GTK3 */
#include <glib.h>
#include <glib/gstdio.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sys/types.h>
#include <dirent.h>
#include <sys/stat.h>
#include <pwd.h>
#include <unistd.h>

#define MAX_MENU_ITEMS 50
#define MAX_PREVIEW_LEN 80
#define MESH_BUCKET_SUBPATH "/.qnm-sync/clipboard"

typedef struct {
    GtkWidget   *button;
    GtkWidget   *plug;
    gchar       *bucket_dir;        /* ~/QNM-Shared/.qnm-sync/clipboard */
    gchar       *last_published_hash;
    guint        watch_timer;
    GtkClipboard *clipboard;
} MackesClipboard;


/* ------------------------------------------------------------------ helpers */

static gchar *
home_path (const gchar *suffix)
{
    const gchar *home = g_get_home_dir ();
    return g_build_filename (home, "QNM-Shared", suffix, NULL);
}

static gchar *
short_hash_of (const gchar *text)
{
    /* djb2 — sufficient for change-detection deduping (NOT cryptographic) */
    guint64 hash = 5381;
    while (*text)
        hash = ((hash << 5) + hash) + (guchar) *text++;
    return g_strdup_printf ("%012lx", hash);
}

static gchar *
get_latest_version_path (const gchar *key_dir)
{
    GDir *dir = g_dir_open (key_dir, 0, NULL);
    if (!dir) return NULL;
    const gchar *fname;
    gint best = 0;
    while ((fname = g_dir_read_name (dir)) != NULL) {
        if (g_str_has_prefix (fname, "v") && g_str_has_suffix (fname, ".dat")) {
            gint n = (gint) g_ascii_strtoll (fname + 1, NULL, 10);
            if (n > best) best = n;
        }
    }
    g_dir_close (dir);
    if (best == 0) return NULL;
    gchar *vname = g_strdup_printf ("v%d.dat", best);
    gchar *full = g_build_filename (key_dir, vname, NULL);
    g_free (vname);
    return full;
}


/* ------------------------------------------------------------ entry listing */

typedef struct {
    gchar  *key;          /* file name relative to bucket dir */
    gchar  *peer;         /* "mine" or other peer's hostname */
    time_t  mtime;
    gchar  *data_path;    /* full path to latest revision */
} ClipEntry;

static void clip_entry_free (gpointer p)
{
    ClipEntry *e = p;
    g_free (e->key);
    g_free (e->peer);
    g_free (e->data_path);
    g_free (e);
}

static gint
clip_entry_cmp_mtime_desc (gconstpointer a, gconstpointer b)
{
    const ClipEntry *ea = a, *eb = b;
    return (eb->mtime > ea->mtime) - (eb->mtime < ea->mtime);
}

/*
 * Recursively walks <bucket-dir>/<peer-hostname>/<key>/v*.dat. We treat
 * the immediate subdir of bucket-dir as the peer's hostname; the next
 * level (a directory) is the entry "key". Inside each key dir live
 * v1.dat / v2.dat / ... versions; we use the largest.
 */
static GList *
load_entries (MackesClipboard *mc)
{
    GList *out = NULL;
    /*
     * mesh-sync layout differs slightly per peer. The Mackes Python module
     * writes into bucket_dir/<key-dir>/vN.dat. Other peers' entries live
     * under ~/QNM-Mesh/<peer>/.qnm-sync/clipboard/<key-dir>/vN.dat after
     * sshfs mount. We scan both layouts.
     */
    const gchar *me = g_get_host_name ();

    /* 1. Local items */
    if (g_file_test (mc->bucket_dir, G_FILE_TEST_IS_DIR)) {
        GDir *d = g_dir_open (mc->bucket_dir, 0, NULL);
        if (d) {
            const gchar *kname;
            while ((kname = g_dir_read_name (d)) != NULL) {
                gchar *key_dir = g_build_filename (mc->bucket_dir, kname, NULL);
                if (g_file_test (key_dir, G_FILE_TEST_IS_DIR)) {
                    gchar *latest = get_latest_version_path (key_dir);
                    if (latest) {
                        struct stat st;
                        if (g_stat (latest, &st) == 0) {
                            ClipEntry *e = g_new0 (ClipEntry, 1);
                            e->key = g_strdup (kname);
                            e->peer = g_strdup ("mine");
                            e->mtime = st.st_mtime;
                            e->data_path = g_steal_pointer (&latest);
                            out = g_list_prepend (out, e);
                        }
                        g_free (latest);
                    }
                }
                g_free (key_dir);
            }
            g_dir_close (d);
        }
    }

    /* 2. Peer items from ~/QNM-Mesh/<peer>/.qnm-sync/clipboard/ */
    gchar *mesh_root = g_build_filename (g_get_home_dir (), "QNM-Mesh", NULL);
    if (g_file_test (mesh_root, G_FILE_TEST_IS_DIR)) {
        GDir *peers = g_dir_open (mesh_root, 0, NULL);
        if (peers) {
            const gchar *peer;
            while ((peer = g_dir_read_name (peers)) != NULL) {
                if (g_strcmp0 (peer, me) == 0) continue;
                gchar *pbucket = g_build_filename (mesh_root, peer,
                                                   ".qnm-sync", "clipboard", NULL);
                if (g_file_test (pbucket, G_FILE_TEST_IS_DIR)) {
                    GDir *d2 = g_dir_open (pbucket, 0, NULL);
                    if (d2) {
                        const gchar *kname;
                        while ((kname = g_dir_read_name (d2)) != NULL) {
                            gchar *kd = g_build_filename (pbucket, kname, NULL);
                            if (g_file_test (kd, G_FILE_TEST_IS_DIR)) {
                                gchar *latest = get_latest_version_path (kd);
                                if (latest) {
                                    struct stat st;
                                    if (g_stat (latest, &st) == 0) {
                                        ClipEntry *e = g_new0 (ClipEntry, 1);
                                        e->key = g_strdup (kname);
                                        e->peer = g_strdup (peer);
                                        e->mtime = st.st_mtime;
                                        e->data_path = g_steal_pointer (&latest);
                                        out = g_list_prepend (out, e);
                                    }
                                    g_free (latest);
                                }
                            }
                            g_free (kd);
                        }
                        g_dir_close (d2);
                    }
                }
                g_free (pbucket);
            }
            g_dir_close (peers);
        }
    }
    g_free (mesh_root);

    out = g_list_sort (out, clip_entry_cmp_mtime_desc);
    return out;
}


/* ----------------------------------------------------------- menu generation */

static gchar *
preview_text_for (const ClipEntry *e)
{
    gchar *contents = NULL;
    gsize len = 0;
    g_file_get_contents (e->data_path, &contents, &len, NULL);
    if (!contents) return g_strdup ("(unreadable)");
    /* trim to 80 chars + newline-collapse */
    if (len > MAX_PREVIEW_LEN) len = MAX_PREVIEW_LEN;
    gchar *buf = g_malloc (len + 1);
    gsize j = 0;
    for (gsize i = 0; i < len; i++) {
        gchar c = contents[i];
        if (c == '\n' || c == '\r' || c == '\t') c = ' ';
        if ((guchar) c < 32 && c != ' ') c = '?';
        buf[j++] = c;
    }
    buf[j] = '\0';
    g_free (contents);
    return buf;
}

static void
on_menu_item_activated (GtkMenuItem *item, gpointer data)
{
    ClipEntry *e = g_object_get_data (G_OBJECT (item), "clip-entry");
    if (!e) return;
    gchar *contents = NULL;
    gsize len = 0;
    if (g_file_get_contents (e->data_path, &contents, &len, NULL)) {
        GtkClipboard *cb = gtk_clipboard_get (GDK_SELECTION_CLIPBOARD);
        gtk_clipboard_set_text (cb, contents, len);
        gtk_clipboard_store (cb);
        g_free (contents);
    }
}

static void
build_menu (GtkMenu *menu, MackesClipboard *mc)
{
    GList *entries = load_entries (mc);
    GList *l;
    int count = 0;

    if (!entries) {
        GtkWidget *empty = gtk_menu_item_new_with_label (
            "(no mesh clipboard items yet)");
        gtk_widget_set_sensitive (empty, FALSE);
        gtk_menu_shell_append (GTK_MENU_SHELL (menu), empty);
        gtk_widget_show (empty);
        return;
    }

    gchar *last_peer = NULL;
    for (l = entries; l != NULL && count < MAX_MENU_ITEMS; l = l->next) {
        ClipEntry *e = l->data;

        /* Section header per peer */
        if (last_peer == NULL || g_strcmp0 (last_peer, e->peer) != 0) {
            if (count > 0) {
                GtkWidget *sep = gtk_separator_menu_item_new ();
                gtk_menu_shell_append (GTK_MENU_SHELL (menu), sep);
                gtk_widget_show (sep);
            }
            gchar *hdr_text = g_strdup_printf ("— %s —", e->peer);
            GtkWidget *hdr = gtk_menu_item_new_with_label (hdr_text);
            gtk_widget_set_sensitive (hdr, FALSE);
            gtk_menu_shell_append (GTK_MENU_SHELL (menu), hdr);
            gtk_widget_show (hdr);
            g_free (hdr_text);
            last_peer = e->peer;
        }

        gchar *prev = preview_text_for (e);
        GtkWidget *item = gtk_menu_item_new_with_label (prev);
        g_free (prev);
        /* Hand ownership of e to the item so it survives until item destroy */
        g_object_set_data_full (G_OBJECT (item), "clip-entry", e,
                                (GDestroyNotify) clip_entry_free);
        l->data = NULL;   /* don't let g_list_free_full free it */
        g_signal_connect (item, "activate",
                          G_CALLBACK (on_menu_item_activated), NULL);
        gtk_menu_shell_append (GTK_MENU_SHELL (menu), item);
        gtk_widget_show (item);
        count++;
    }

    /* Free the list shell + any leftover entries we didn't transfer */
    g_list_free_full (entries, clip_entry_free);
}


/* ------------------------------------------------------------ click + popup */

static void
on_button_clicked (GtkButton *button, gpointer data)
{
    MackesClipboard *mc = data;
    GtkMenu *menu = GTK_MENU (gtk_menu_new ());
    build_menu (menu, mc);
    g_signal_connect (menu, "selection-done",
                      G_CALLBACK (gtk_widget_destroy), NULL);
    gtk_menu_popup_at_widget (menu, GTK_WIDGET (button),
                              GDK_GRAVITY_SOUTH_WEST,
                              GDK_GRAVITY_NORTH_WEST, NULL);
}


/* ------------------------------------------------- selection watcher (write) */

static gboolean
publish_current_clipboard (gpointer data)
{
    MackesClipboard *mc = data;
    gchar *text = gtk_clipboard_wait_for_text (mc->clipboard);
    if (!text || *text == '\0') {
        g_free (text);
        return G_SOURCE_CONTINUE;
    }

    gchar *h = short_hash_of (text);
    if (mc->last_published_hash && g_strcmp0 (mc->last_published_hash, h) == 0) {
        g_free (h);
        g_free (text);
        return G_SOURCE_CONTINUE;
    }
    g_free (mc->last_published_hash);
    mc->last_published_hash = h;

    /* Build key dir: <bucket>/<ISO-ts>_<short-hash>.txt/v1.dat */
    time_t now = time (NULL);
    struct tm tm;
    localtime_r (&now, &tm);
    gchar ts[32];
    strftime (ts, sizeof ts, "%Y-%m-%dT%H-%M-%S", &tm);

    gchar *short_h = g_strndup (h, 6);
    gchar *key_name = g_strdup_printf ("%s_%s.txt", ts, short_h);
    gchar *key_dir = g_build_filename (mc->bucket_dir, key_name, NULL);
    g_free (short_h);
    g_free (key_name);

    g_mkdir_with_parents (key_dir, 0755);
    gchar *out = g_build_filename (key_dir, "v1.dat", NULL);
    g_file_set_contents (out, text, -1, NULL);

    /* "latest" symlink */
    gchar *latest = g_build_filename (key_dir, "latest", NULL);
    g_unlink (latest);
    g_free (latest);

    g_free (out);
    g_free (key_dir);
    g_free (text);
    return G_SOURCE_CONTINUE;
}


/* ----------------------------------------------------------------- bootstrap */

int main (int argc, char **argv)
{
    gtk_init (&argc, &argv);

    /* Parse --socket-id (xfce4-panel external-plugin contract) */
    Window socket_id = 0;
    for (int i = 1; i < argc; i++) {
        if (g_str_has_prefix (argv[i], "--socket-id=")) {
            socket_id = (Window) g_ascii_strtoull (argv[i] + 12, NULL, 10);
        } else if (g_strcmp0 (argv[i], "--socket-id") == 0 && i + 1 < argc) {
            socket_id = (Window) g_ascii_strtoull (argv[++i], NULL, 10);
        }
    }

    GtkWidget *plug = gtk_plug_new (socket_id);
    GtkWidget *button = gtk_button_new ();
    GtkWidget *image = gtk_image_new_from_icon_name (
        "edit-paste", GTK_ICON_SIZE_LARGE_TOOLBAR);
    gtk_button_set_image (GTK_BUTTON (button), image);
    gtk_button_set_relief (GTK_BUTTON (button), GTK_RELIEF_NONE);
    gtk_widget_set_tooltip_text (button, "Mackes Mesh Clipboard");
    gtk_container_add (GTK_CONTAINER (plug), button);

    MackesClipboard *mc = g_new0 (MackesClipboard, 1);
    mc->button = button;
    mc->plug = plug;
    mc->bucket_dir = home_path (".qnm-sync/clipboard");
    g_mkdir_with_parents (mc->bucket_dir, 0755);
    mc->clipboard = gtk_clipboard_get (GDK_SELECTION_CLIPBOARD);
    mc->last_published_hash = NULL;

    g_signal_connect (button, "clicked", G_CALLBACK (on_button_clicked), mc);
    g_signal_connect (mc->clipboard, "owner-change",
                      G_CALLBACK (publish_current_clipboard), mc);
    /* Periodic flush in case owner-change signal misses */
    mc->watch_timer = g_timeout_add_seconds (5, publish_current_clipboard, mc);

    g_signal_connect (plug, "destroy", G_CALLBACK (gtk_main_quit), NULL);
    gtk_widget_show_all (plug);
    gtk_main ();

    g_free (mc->bucket_dir);
    g_free (mc->last_published_hash);
    g_free (mc);
    return 0;
}
