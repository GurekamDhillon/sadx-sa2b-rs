//! Character controller for the Sonic Adventure engine.
//!
//! [`motion`] holds the engine's physics functions (acceleration, turning,
//! speed integration) decoded from SADX and driven by `PhysicsData` (read
//! from SA2 or SADX with `sa_data`). [`player`] adds the action state machine
//! (stand/run/jump/fall/hurt), and [`world`] is the collision interface the
//! host engine implements.

pub mod angle;
pub mod frame;
pub mod motion;
pub mod player;
pub mod vec;
pub mod world;

pub use motion::{Body, Stick};
pub use player::{Action, Input, Player};
pub use vec::Vec3;
pub use world::{FlatWorld, Ground, MeshWorld, World};
