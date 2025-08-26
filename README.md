# brickrail-rs
Automation software for LEGO PoweredUp based train layouts running on a PC. Reimplementation of [brickrail](https://github.com/Novakasa/brickrail) in rust.

This software allows you to recreate the topology your LEGO train layout virtually and dispatch your LEGO trains automatically with path finding, collision avoidance and programmed speeds.
Compatible with LEGO PoweredUp hubs, LEGO Control+ hubs and the LEGO Robot Inventor Hub. Requires Pybricks firmware running on the hubs.
LEGO Color & Distance sensors on the trains are used to report train positions to brickrail.

## Comparison to original project
This project is a work-in-progress reimplementation of [brickrail](https://github.com/Novakasa/brickrail) in rust, using the bevy game engine as a framework.
It profits from a lot of hindsight from the original implementation with vastly different and improved internal architecture that makes implementation of new features much easier.
This project does not require a seperate python process to be running in parallel thanks to a custom implementation of the Pybricks protocol in rust. This hopefully makes it easier to deploy on MacOS in the future.

### Added features
- More intuitive marker placements. Dropped requirement for manually defining "Reverse entry markers". Each marker that precedes a block "in" marker is automatically considered as the "enter" marker.
- Multiple motors per train with configurable polarity/direction.  
- Puppet hubs for trains. This allows for double-headed trains, mostly useful for adding more motors and distributed power.
- Schedule system. Allows train to follow a schedule that travels between "Destinations" which can comprise of multiple blocks, modeling the concept of "station platforms".

### Missing features (Roadmap)
- Overall polish is not yet a focus of this project, so visuals and user experience is not as clean (yet)
- Level crossings not implemented yet (will be added)
- Documentation (wiki) missing entirely (for now)
- We are investigating using an advertising mode to communicate with stationary hubs, allowing to push beyond the usual 7-8 max connection limit of bluetooth.