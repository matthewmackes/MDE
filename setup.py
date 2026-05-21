"""setup.py — for sdist tarball generation used by the RPM build.

Mackes installs its actual data (presets, wallpapers, desktop entry, icon)
via the .spec file's %install section, not via package_data, because the
canonical install path is /usr/share/mackes-shell/, not inside the Python
sitelib.
"""
from __future__ import annotations

from setuptools import find_packages, setup


setup(
    name="mackes-shell",
    version="2.0.1",
    description="Mackes Desktop Environment (MDE) — Wayland-only Fedora DE (v2.0.0 cut; PyPI name kept for one-release back-compat)",
    long_description=(
        "GTK3 / PyGObject control panel that replaces xfce4-settings as the "
        "daily interface on Fedora XFCE workstations. Standard XFCE shell "
        "underneath (Whisker Menu + xfce4-panel + xfdesktop), styled with "
        "the Carbon Design System."
    ),
    long_description_content_type="text/plain",
    author="Matt Mackes",
    author_email="matthewmackes@gmail.com",
    url="https://github.com/mattmacke/mackes-shell",
    license="GPL-3.0",
    # v2.0.0 Phase 0.10 — `mde` ships as a thin re-export facade
    # over `mackes` during the back-compat window. The facade
    # itself is a one-file package (no submodules in-tree) but the
    # package needs to be listed so setuptools installs the
    # __init__.py that aliases sys.modules at import time.
    packages=find_packages(include=["mackes", "mackes.*", "mde"]),
    python_requires=">=3.10",
    install_requires=["PyYAML"],
    entry_points={
        "console_scripts": [
            "mackes = mackes.app:main",
        ],
    },
    classifiers=[
        "Development Status :: 4 - Beta",
        "Environment :: X11 Applications :: GTK",
        "Intended Audience :: End Users/Desktop",
        "License :: OSI Approved :: GNU General Public License v3 (GPLv3)",
        "Operating System :: POSIX :: Linux",
        "Programming Language :: Python :: 3",
        "Topic :: Desktop Environment",
    ],
)
