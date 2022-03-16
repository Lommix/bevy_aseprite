use std::time::Duration;

use bevy::prelude::*;

use crate::{Aseprite, AsepriteInfo};
use bevy_aseprite_reader as reader;

/// A tag representing an animation
#[derive(Debug, Default, Component, Copy, Clone, PartialEq, Eq)]
pub struct AsepriteTag(&'static str);

impl std::ops::Deref for AsepriteTag {
    type Target = &'static str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsepriteTag {
    /// Create a new tag
    pub const fn new(id: &'static str) -> AsepriteTag {
        AsepriteTag(id)
    }
}

#[derive(Debug, Default, Component, PartialEq, Eq)]
pub struct AsepriteAnimation {
    pub is_playing: bool,
    pub tag: Option<&'static str>,
    pub current_frame: usize,
    pub forward: bool,
    pub time_elapsed: Duration,
    pub tag_changed: bool,
}

impl AsepriteAnimation {
    pub fn tag(tag: &'static str) -> Self {
        Self {
            tag: Some(tag),
            ..Default::default()
        }
    }

    /// Return the first frame of the tag or 0 if no tag
    pub fn get_first_frame(&self, info: &AsepriteInfo) -> usize {
        match self.tag {
            Some(tag) => {
                let tag = match info.tags.get(tag) {
                    Some(tag) => tag,
                    None => {
                        error!("Tag {} wasn't found.", tag);
                        return 0;
                    }
                };

                let range = tag.frames.clone();
                range.start as usize
            }
            _ => 0,
        }
    }

    fn next_frame(&mut self, info: &AsepriteInfo) {
        match self.tag {
            Some(tag) => {
                let tag = match info.tags.get(tag) {
                    Some(tag) => tag,
                    None => {
                        error!("Tag {} wasn't found.", tag);
                        return;
                    }
                };

                let range = tag.frames.clone();
                dbg!(&range);
                match tag.animation_direction {
                    reader::raw::AsepriteAnimationDirection::Forward => {
                        let next_frame = self.current_frame + 1;
                        if range.contains(&(next_frame as u16)) {
                            self.current_frame = next_frame;
                        } else {
                            self.current_frame = range.start as usize;
                        }
                    }
                    reader::raw::AsepriteAnimationDirection::Reverse => {
                        let next_frame = self.current_frame.checked_sub(1);
                        if let Some(next_frame) = next_frame {
                            if range.contains(&((next_frame) as u16)) {
                                self.current_frame = next_frame;
                            } else {
                                self.current_frame = range.end as usize - 1;
                            }
                        } else {
                            // TODO check -1 is correct
                            self.current_frame = range.end as usize - 1;
                        }
                    }
                    reader::raw::AsepriteAnimationDirection::PingPong => {
                        if self.forward {
                            let next_frame = self.current_frame + 1;
                            if range.contains(&(next_frame as u16)) {
                                self.current_frame = next_frame;
                            } else {
                                self.current_frame = next_frame.saturating_sub(1);
                                self.forward = false;
                            }
                        } else {
                            let next_frame = self.current_frame.checked_sub(1);
                            if let Some(next_frame) = next_frame {
                                if range.contains(&(next_frame as u16)) {
                                    self.current_frame = next_frame
                                }
                            }
                            self.current_frame += 1;
                            self.forward = true;
                        }
                    }
                }
            }
            None => {
                dbg!(self.current_frame, info.frame_count);
                self.current_frame = (self.current_frame + 1) % info.frame_count;
            }
        }
    }

    pub fn current_frame_duration(&self, info: &AsepriteInfo) -> Duration {
        // TODO store delay ms as Durations?
        Duration::from_millis(info.frame_infos[self.current_frame].delay_ms as u64)
    }

    pub fn update(&mut self, info: &AsepriteInfo, dt: Duration) -> bool {
        self.time_elapsed += dt;
        let mut current_frame_duration = self.current_frame_duration(info);
        let mut frame_changed = false;
        while self.time_elapsed >= current_frame_duration {
            self.time_elapsed -= current_frame_duration;
            self.next_frame(info);
            current_frame_duration = self.current_frame_duration(info);
            frame_changed = true;
        }
        dbg!(
            dt,
            self.time_elapsed,
            current_frame_duration,
            self.current_frame,
            frame_changed
        );
        frame_changed
    }

    /// Get the current frame
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    /// Start or resume playing an animation
    pub fn play(&mut self) {
        self.is_playing = true;
    }

    /// Pause the current animation
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// Returns `true` if the animation is playing
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Returns `true` if the animation is paused
    pub fn is_paused(&self) -> bool {
        !self.is_playing
    }

    /// Toggle state between playing and pausing
    pub fn toggle(&mut self) {
        self.is_playing = !self.is_playing;
    }
}

pub(crate) fn update_animations(
    time: Res<Time>,
    aseprites: Res<Assets<Aseprite>>,
    atlases: Res<Assets<TextureAtlas>>,
    mut aseprites_query: Query<(
        &Handle<Aseprite>,
        &mut AsepriteAnimation,
        &mut TextureAtlasSprite,
    )>,
) {
    for (handle, mut animation, mut sprite) in aseprites_query.iter_mut() {
        let aseprite = match aseprites.get(handle) {
            Some(aseprite) => aseprite,
            None => {
                error!("Aseprite handle is invalid");
                continue;
            }
        };
        let info = match &aseprite.info {
            Some(info) => info,
            None => {
                error!("Aseprite info is None");
                continue;
            }
        };
        let atlas_handle = match &aseprite.atlas {
            Some(handle) => handle,
            None => {
                error!("Aseprite atlas is None");
                continue;
            }
        };
        let atlas = match atlases.get(atlas_handle) {
            Some(atlas) => atlas,
            None => {
                error!("Aseprite atlas is None");
                continue;
            }
        };
        if animation.update(info, time.delta()) {
            sprite.index = atlas.get_texture_index(&aseprite.frame_handles[animation.current_frame]).unwrap();
        }
    }
}
