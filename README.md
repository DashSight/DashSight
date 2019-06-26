# DashSight

DashSight is a tool for collecting and displaying in real time information about your car and your driving. DashSight uses GPS and OBDII data from your car to give you insights into your driving. All of this is collected and displayed in realtime.

Although DashSight is written using standard glib and GTK+3 APIs it is designed to run on specialised hardware. DashSight can be ported to other hardware or edited to be more generic with limited work. This is something which hopefully will happen in the future.

## Dependencies

* gpsd
  * DashSight uses GPSD to communicate with the GPS device. This means it should work with almost any GPS device and is portable to most Linux systems.
* gtk+3 and glib-2
  * DashSight uses GTK+3 for graphic display and glib for helper functions
* osm-gps-map
  * DashSight uses osm-gps-map for the map rendering.
* [Python-OBD](https://github.com/brendan-w/python-OBD) and Python3
  * DashSight relies on [Python-OBD](https://github.com/brendan-w/python-OBD) for the OBD communication.

## Current features

* Ability to record tracks in the standard GPX format
* Ability to download high details maps of a track for offline use
* Ability to free drive
  * DashSigh will show current and previous locations on a map
  * Will show engine revs, throttle position, engine load, fluid temperatures, timing advance and more in real time while driving.
* Ability to load a saved map and drive on that
  * Supports the same features as free drive plus:
    * Displays a high accuracy timer that starts when you cross the track start point and stops when you cross the stop point

## Future features

* Ability to support a circular track, a track with the same start/stop point and multiple laps.
* Ability to display the current speed?
* Replace osm-gps-map with something that is maintained
* Make the code more portable so it can run on any system
* Rewrite the entire thing in Rust and Qt5/GTK4?

## Using DashSight

Currently DashSight only runs on the following specilised hardware.

DashSight runs on the Pine64 Single Board Computer (SBC). This provides the CPU, GPU and WiFi for the device. DashSight then uses an expansion board to add GPS and OBDII to the Pine64 SBC. The schematics and design for the board can be found in the [DashSight Mezzanine Card repo](https://github.com/alistair23/DashSight-Mezzanine-Card)

You can see a production run of the board below:

![DashSight Mezzanine Card Photo 3](https://github.com/alistair23/DashSight-Mezzanine-Card/blob/master/PCB-Fabrication/v2.0-A/Photos/Photo3.jpg "DashSight Mezzanine Card Photo 3")

DashSight uses Yocto/OpenEmbedded to build images that can be directly deployed to the board. This uses the [meta-pine64 layer](https://github.com/alistair23/meta-pine64.git) and the [meta-dashsight layer](https://github.com/alistair23/meta-dashsight).
