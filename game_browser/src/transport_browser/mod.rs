use kay::{World, ActorSystem, Actor, RawID, External, TypedID};
use compact::CVec;
use std::collections::HashMap;
use descartes::LinePath;
use michelangelo::{MeshGrouper, Instance};
use browser_utils::{FrameListener, FrameListenerID, flatten_instances, updated_groups_to_js};

#[derive(Compact, Clone)]
pub struct BrowserTransportUI {
    id: BrowserTransportUIID,
    state: External<BrowserTransportUINonPersistedState>,
}

impl ::std::ops::Deref for BrowserTransportUI {
    type Target = BrowserTransportUINonPersistedState;

    fn deref(&self) -> &BrowserTransportUINonPersistedState {
        &self.state
    }
}

impl ::std::ops::DerefMut for BrowserTransportUI {
    fn deref_mut(&mut self) -> &mut BrowserTransportUINonPersistedState {
        &mut self.state
    }
}

pub struct BrowserTransportUINonPersistedState {
    car_instance_buffers: HashMap<RawID, Vec<::michelangelo::Instance>>,

    // transport geometry
    asphalt_grouper: MeshGrouper<RawID>,
    lane_marker_grouper: MeshGrouper<RawID>,
    lane_marker_gaps_grouper: MeshGrouper<RawID>,
}

impl BrowserTransportUI {
    pub fn spawn(id: BrowserTransportUIID, world: &mut World) -> BrowserTransportUI {
        {
            ::transport::lane::LaneID::global_broadcast(world).get_render_info(id.into(), world);
            ::transport::lane::SwitchLaneID::global_broadcast(world)
                .get_render_info(id.into(), world);
        }

        BrowserTransportUI {
            id,
            state: External::new(BrowserTransportUINonPersistedState {
                car_instance_buffers: HashMap::new(),
                asphalt_grouper: MeshGrouper::new(2000),
                lane_marker_grouper: MeshGrouper::new(2000),
                lane_marker_gaps_grouper: MeshGrouper::new(2000),
            }),
        }
    }
}

impl FrameListener for BrowserTransportUI {
    fn on_frame(&mut self, world: &mut World) {
        ::transport::lane::LaneID::global_broadcast(world).get_car_instances(self.id_as(), world);
        ::transport::lane::SwitchLaneID::global_broadcast(world)
            .get_car_instances(self.id_as(), world);

        let mut car_instances = Vec::with_capacity(600_000);

        for lane_instances in self.car_instance_buffers.values() {
            car_instances.extend_from_slice(lane_instances);
        }

        let car_instances_js: ::stdweb::web::TypedArray<f32> =
            flatten_instances(&car_instances).into();

        js! {
            window.cbReactApp.setState(oldState => update(oldState, {
                transport: {rendering: {
                    carInstances: {"$set": @{car_instances_js}}
                }}
            }))
        }
    }
}

use transport::ui::{TransportUI, TransportUIID};

impl TransportUI for BrowserTransportUI {
    fn on_lane_constructed(
        &mut self,
        id: RawID,
        lane_path: &LinePath,
        is_switch: bool,
        on_intersection: bool,
        _world: &mut World,
    ) {
        use ::transport::ui::{lane_mesh, marker_mesh, switch_marker_gap_mesh};
        if is_switch {
            let updated_lane_marker_gaps_groups = self
                .lane_marker_gaps_grouper
                .update(None, Some((id, switch_marker_gap_mesh(lane_path))));

            js!{
                window.cbReactApp.setState(oldState => update(oldState, {
                    transport: {rendering: {
                        laneMarkerGapGroups: {
                            "$add": @{updated_groups_to_js(
                                updated_lane_marker_gaps_groups
                            )}
                        }
                    }}
                }));
            }
        } else {
            let mesh = lane_mesh(lane_path);
            let updated_asphalt_groups = self.asphalt_grouper.update(None, Some((id, mesh)));

            if on_intersection {
                js!{
                    window.cbReactApp.setState(oldState => update(oldState, {
                        transport: {rendering: {
                            laneAsphaltGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_asphalt_groups
                                )}
                            }
                        }}
                    }));
                }
            } else {
                let marker_meshes = marker_mesh(lane_path);
                let updated_lane_marker_groups = self
                    .lane_marker_grouper
                    .update(None, Some((id, marker_meshes.0 + marker_meshes.1)));
                js!{
                    window.cbReactApp.setState(oldState => update(oldState, {
                        transport: {rendering: {
                            laneAsphaltGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_asphalt_groups
                                )}
                            },
                            laneMarkerGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_lane_marker_groups
                                )}
                            }
                        }}
                    }));
                }
            }
        }
    }

    fn on_lane_destructed(
        &mut self,
        id: RawID,
        is_switch: bool,
        on_intersection: bool,
        _world: &mut World,
    ) {
        if is_switch {
            let updated_lane_marker_gaps_groups =
                self.lane_marker_gaps_grouper.update(Some(id), None);

            js!{
                window.cbReactApp.setState(oldState => update(oldState, {
                    transport: {rendering: {
                        laneMarkerGapGroups: {
                            "$add": @{updated_groups_to_js(
                                updated_lane_marker_gaps_groups
                            )}
                        }
                    }}
                }));
            }
        } else {
            let updated_asphalt_groups = self.asphalt_grouper.update(Some(id), None);

            if on_intersection {
                js!{
                    window.cbReactApp.setState(oldState => update(oldState, {
                        transport: {rendering: {
                            laneAsphaltGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_asphalt_groups
                                )}
                            }
                        }}
                    }));
                }
            } else {
                let updated_lane_marker_groups = self.lane_marker_grouper.update(Some(id), None);
                js!{
                    window.cbReactApp.setState(oldState => update(oldState, {
                        transport: {rendering: {
                            laneAsphaltGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_asphalt_groups
                                )}
                            },
                            laneMarkerGroups: {
                                "$add": @{updated_groups_to_js(
                                    updated_lane_marker_groups
                                )}
                            }
                        }}
                    }));
                }
            }
        }
    }

    fn on_car_instances(&mut self, from_lane: RawID, instances: &CVec<Instance>, _: &mut World) {
        self.car_instance_buffers
            .insert(from_lane, instances.to_vec());
    }
}

mod kay_auto;
pub use self::kay_auto::*;

pub fn setup(system: &mut ActorSystem) {
    system.register::<BrowserTransportUI>();
    auto_setup(system);
}

pub fn spawn(world: &mut World) {
    BrowserTransportUIID::spawn(world);
}
