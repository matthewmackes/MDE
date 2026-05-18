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
    version="1.0.0",
    description="Mackes Shell — XFCE control panel and shell manager",
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
    packages=find_packages(include=["mackes", "mackes.*"]),
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
