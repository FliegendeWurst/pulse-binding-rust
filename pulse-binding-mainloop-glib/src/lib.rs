// Copyright 2017 Lyndon Brown
//
// This file is part of the PulseAudio Rust language binding.
//
// Licensed under the MIT license or the Apache license (version 2.0), at your option. You may not
// copy, modify, or distribute this file except in compliance with said license. You can find copies
// of these licenses either in the LICENSE-MIT and LICENSE-APACHE files, or alternatively at
// <http://opensource.org/licenses/MIT> and <http://www.apache.org/licenses/LICENSE-2.0>
// respectively.
//
// Portions of documentation are copied from the LGPL 2.1+ licensed PulseAudio C headers on a
// fair-use basis, as discussed in the overall project readme (available in the git repository).

//! PulseAudio Rust language binding library for the ‘GLIB mainloop’ component.
//!
//! This component provides a wrapper around the GLIB main loop. Use this to embed PulseAudio into
//! your GLIB/GTK+/GNOME programs.
//!
//! # About
//!
//! This library is a binding that allows Rust code to connect to the PulseAudio sound server via
//! PulseAudio’s existing C API. This binding provides a safe(r) Rust interface which might be
//! preferred over the raw C API provided by the underlying `sys` linking crate.
//!
//! This crate provides an interface to PulseAudio’s ‘GLIB mainloop’ component, and should be used
//! in addition to the general `libpulse_binding` crate.
//!
//! # Introduction
//!
//! The GLIB main loop bindings are extremely easy to use. All that is required is to create a
//! Mainloop object using [`Mainloop::new`]. When the main loop abstraction is needed, it is
//! provided by [`Mainloop::get_api`].
//!
//! # Usage
//!
//! Firstly, add a dependency on the crate in your program’s `Cargo.toml` file. Secondly, import the
//! crate along with the general `libpulse_binding` crate to the root of your program:
//!
//! ```rust,ignore
//! extern crate libpulse_binding as pulse;
//! extern crate libpulse_glib_binding as pglib;
//! ```
//!
//! See the documentation in `libpulse_binding` for further information regarding actual usage of
//! libpulse mainloops.
//!
//! [`Mainloop::new`]: struct.Mainloop.html#method.new
//! [`Mainloop::get_api`]: struct.Mainloop.html#method.get_api

#![doc(html_logo_url = "https://github.com/jnqnfe/pulse-binding-rust/raw/master/logo.png",
       html_favicon_url = "https://github.com/jnqnfe/pulse-binding-rust/raw/master/favicon.ico")]

extern crate libpulse_binding as pulse;
extern crate libpulse_mainloop_glib_sys as capi;

use std::rc::Rc;
use std::ptr::{null, null_mut};
use glib_sys::GMainContext;
use glib::{MainContext, translate::ToGlibPtr};
use std::mem;
use pulse::mainloop::api::{MainloopInternalType, MainloopInner, MainloopApi};
use pulse::mainloop::signal::MainloopSignals;
use pulse::mainloop::api::Mainloop as MainloopTrait;

/* Note, we cannot simply use the object defined in the ‘sys’ crate, since either the type or the
 * trait need to be defined locally in order to link them. Thus, we create the below type (an empty
 * one, just used as an opaque pointer), and transmute to the ‘sys’ crate one.
 */

/// An opaque GLIB main loop object.
#[repr(C)] pub struct MainloopInternal { _private: [u8; 0] }

impl MainloopInternalType for MainloopInternal {}

/// This acts as a safe interface to the internal PA Mainloop.
///
/// The mainloop object pointers are further enclosed here in a ref counted wrapper, allowing this
/// outer wrapper to have clean methods for creating event objects, which can cleanly pass a copy of
/// the inner ref counted mainloop object to them. Giving this to events serves two purposes,
/// firstly because they need the API pointer, secondly, it ensures that event objects do not
/// outlive the mainloop object.
pub struct Mainloop {
    /// The ref-counted inner data.
    pub _inner: Rc<MainloopInner<MainloopInternal>>,
}

impl MainloopTrait for Mainloop {
    type MI = MainloopInner<MainloopInternal>;

    fn inner(&self) -> Rc<MainloopInner<MainloopInternal>> {
        Rc::clone(&self._inner)
    }
}

impl MainloopSignals for Mainloop {}

/// Drop function for MainloopInner<MainloopInternal>.
fn drop_actual(self_: &mut MainloopInner<MainloopInternal>) {
    unsafe { capi::pa_glib_mainloop_free(mem::transmute(&self_.ptr)) };
    self_.ptr = null_mut::<MainloopInternal>();
    self_.api = null::<MainloopApi>();
}

impl Mainloop {
    /// Creates a new GLIB main loop object for the specified GLIB main loop context.
    ///
    /// Takes an argument `context` for the `glib::MainContext` to use. If context is `None` the
    /// default context is used.
    ///
    /// This returns the object in an Rc wrapper, allowing multiple references to be held, which
    /// allows event objects to hold one, thus ensuring they do not outlive it.
    pub fn new(context: Option<&mut MainContext>) -> Option<Self> {
        let p_ctx = context.map_or(null_mut::<GMainContext>(), |c| c.to_glib_none().0);

        let ptr = unsafe { capi::pa_glib_mainloop_new(p_ctx) };
        if ptr.is_null() {
            return None;
        }
        let api_ptr = unsafe {
            mem::transmute(capi::pa_glib_mainloop_get_api(ptr))
        };
        Some(
            Self {
                _inner: Rc::new(
                    MainloopInner::<MainloopInternal> {
                        ptr: unsafe { mem::transmute(ptr) },
                        api: api_ptr,
                        dropfn: drop_actual,
                        supports_rtclock: false,
                    }
                ),
            }
        )
    }

    /// Gets the abstract main loop abstraction layer vtable for this main loop.
    ///
    /// No need to free the API as it is owned by the loop and is destroyed when the loop is freed.
    ///
    /// Talking to PA directly with C requires fetching this pointer explicitly via this function.
    /// This is actually unnecessary through this binding. The pointer is retrieved automatically
    /// upon Mainloop creation, stored internally, and automatically obtained from it by functions
    /// that need it.
    pub fn get_api<'a>(&self) -> &'a MainloopApi {
        let ptr = (*self._inner).api;
        assert_eq!(false, ptr.is_null());
        unsafe { &*ptr }
    }
}
