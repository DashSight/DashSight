# DashSight

DashSight is a tool for collecting and displaying in real time information about your car and your driving. DashSight uses GPS and OBDII data from your car to give you insights into your driving. All of this is collected and displayed in realtime.

DashSight runs on any standard Linux environment. It also run on specialised hardware using a Pine64 board, touch screen and mezzanine card.

## Dependencies

* Rust
  * Rust is awesome and this is written in Rust!
* gpsd
  * DashSight uses GPSD to communicate with the GPS device.
* gtk+3 and glib-2
  * DashSight uses GTK+3 for graphic display and glib for helper functions. This is done thanks to gtk-rs.org/
* libchamplain, Clutter
  * DashSight uses [libchamplain](https://wiki.gnome.org/Projects/libchamplain/) for the map rendering. libchamplain depends on Clutter
* [Python-OBD](https://github.com/brendan-w/python-OBD) and Python3
  * DashSight relies on [Python-OBD](https://github.com/brendan-w/python-OBD) for the OBDII communication.
* libiio
  * libiio is used to access the acceleration data and other sensors.

## Current features

* Ability to record tracks in the standard GPX format
* Read engine revs, throttle position, engine load, fluid temperatures, timing advance and more while driving.
* Ability to load a saved map and drive on that
  * Displays a high accuracy timer that starts when you cross the track start point and stops when you cross the stop point
  * The results can be exported for later analysis

## Using DashSight

### Running on a Linux device

DashSight can run on any Linux device. This includes Linux phones such as the PinePhone.

Installing dependencies

```
pacman -Sy libchamplain libiio python
```

### Running on the specialised board

Currently DashSight runs best on open source [specilised hardware](https://github.com/DashSight/Pine64-Mezzanine-Card/wiki/Bill-of-Materials).

DashSight runs on the Pine64 Single Board Computer (SBC). This provides the CPU, GPU and WiFi for the device. DashSight then uses an expansion board to add GPS and OBDII to the Pine64 SBC. The schematics and design for the board can be found in the [DashSight Mezzanine Card repo](https://github.com/DashSight/DashSight-Mezzanine-Card)

DashSight uses Yocto/OpenEmbedded to build images that can be directly deployed to the board. This uses the [meta-pine64 layer](https://github.com/alistair23/meta-pine64.git) and the [meta-dashsight layer](https://github.com/DashSight/meta-dashsight).
