// Copyright (C) 2019 Alibaba Cloud Computing. All rights reserved.
// SPDX-License-Identifier: Apache-2.0 or BSD-3-Clause
//
// Portions Copyright 2018 Amazon.com, Inc. or its affiliates. All Rights Reserved.
//
// Portions Copyright 2017 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE-BSD-Google file.

//! Common traits and structs for vhost-kern and vhost-user backend drivers.

use super::Result;
use std::os::unix::io::RawFd;
use vmm_sys_util::eventfd::EventFd;

/// Maximum number of memory regions supported.
pub const VHOST_MAX_MEMORY_REGIONS: usize = 255;

/// Vring/virtque configuration data.
pub struct VringConfigData {
    /// Maximum queue size supported by the driver.
    pub queue_max_size: u16,
    /// Actual queue size negotiated by the driver.
    pub queue_size: u16,
    /// Bitmask of vring flags.
    pub flags: u32,
    /// Descriptor table address.
    pub desc_table_addr: u64,
    /// Used ring buffer address.
    pub used_ring_addr: u64,
    /// Available ring buffer address.
    pub avail_ring_addr: u64,
    /// Optional address for logging.
    pub log_addr: Option<u64>,
}

impl VringConfigData {
    /// Check whether the log (flag, address) pair is valid.
    pub fn is_log_addr_valid(&self) -> bool {
        if self.flags & 0x1 != 0 && self.log_addr.is_none() {
            return false;
        }

        true
    }

    /// Get the log address, default to zero if not available.
    pub fn get_log_addr(&self) -> u64 {
        if self.flags & 0x1 != 0 && self.log_addr.is_some() {
            self.log_addr.unwrap()
        } else {
            0
        }
    }
}

/// Memory region configuration data.
#[derive(Default, Clone, Copy)]
pub struct VhostUserMemoryRegionInfo {
    /// Guest physical address of the memory region.
    pub guest_phys_addr: u64,
    /// Size of the memory region.
    pub memory_size: u64,
    /// Virtual address in the current process.
    pub userspace_addr: u64,
    /// Optional offset where region starts in the mapped memory.
    pub mmap_offset: u64,
    /// Optional file diescriptor for mmap
    pub mmap_handle: RawFd,
}

/// An interface for setting up vhost-based backend drivers.
///
/// Vhost devices are subset of virtio devices, which improve virtio device's performance by
/// delegating data plane operations to dedicated IO service processes. Vhost devices use the
/// same virtqueue layout as virtio devices to allow vhost devices to be mapped directly to
/// virtio devices.
/// The purpose of vhost is to implement a subset of a virtio device's functionality outside the
/// VMM process. Typically fast paths for IO operations are delegated to the dedicated IO service
/// processes, and slow path for device configuration are still handled by the VMM process. It may
/// also be used to control access permissions of virtio backend devices.
pub trait VhostBackend: std::marker::Sized {
    /// Get a bitmask of supported virtio/vhost features.
    fn get_features(&mut self) -> Result<u64>;

    /// Inform the vhost subsystem which features to enable.
    /// This should be a subset of supported features from get_features().
    ///
    /// # Arguments
    /// * `features` - Bitmask of features to set.
    fn set_features(&mut self, features: u64) -> Result<()>;

    /// Set the current process as the owner of the vhost backend.
    /// This must be run before any other vhost commands.
    fn set_owner(&mut self) -> Result<()>;

    /// Used to be sent to request disabling all rings
    /// This is no longer used.
    fn reset_owner(&mut self) -> Result<()>;

    /// Set the guest memory mappings for vhost to use.
    fn set_mem_table(&mut self, regions: &[VhostUserMemoryRegionInfo]) -> Result<()>;

    /// Set base address for page modification logging.
    fn set_log_base(&mut self, base: u64, fd: Option<RawFd>) -> Result<()>;

    /// Specify an eventfd file descriptor to signal on log write.
    fn set_log_fd(&mut self, fd: RawFd) -> Result<()>;

    /// Set the number of descriptors in the vring.
    ///
    /// # Arguments
    /// * `queue_index` - Index of the queue to set descriptor count for.
    /// * `num` - Number of descriptors in the queue.
    fn set_vring_num(&mut self, queue_index: usize, num: u16) -> Result<()>;

    /// Set the addresses for a given vring.
    ///
    /// # Arguments
    /// * `queue_index` - Index of the queue to set addresses for.
    /// * `config_data` - Configuration data for a vring.
    fn set_vring_addr(&mut self, queue_index: usize, config_data: &VringConfigData) -> Result<()>;

    /// Set the first index to look for available descriptors.
    ///
    /// # Arguments
    /// * `queue_index` - Index of the queue to modify.
    /// * `num` - Index where available descriptors start.
    fn set_vring_base(&mut self, queue_index: usize, base: u16) -> Result<()>;

    /// Get the available vring base offset.
    fn get_vring_base(&mut self, queue_index: usize) -> Result<u32>;

    /// Set the eventfd to trigger when buffers have been used by the host.
    ///
    /// # Arguments
    /// * `queue_index` - Index of the queue to modify.
    /// * `fd` - EventFd to trigger.
    fn set_vring_call(&mut self, queue_index: usize, fd: &EventFd) -> Result<()>;

    /// Set the eventfd that will be signaled by the guest when buffers are
    /// available for the host to process.
    ///
    /// # Arguments
    /// * `queue_index` - Index of the queue to modify.
    /// * `fd` - EventFd that will be signaled from guest.
    fn set_vring_kick(&mut self, queue_index: usize, fd: &EventFd) -> Result<()>;

    /// Set the eventfd that will be signaled by the guest when error happens.
    ///
    /// # Arguments
    /// * `queue_index` - Index of the queue to modify.
    /// * `fd` - EventFd that will be signaled from guest.
    fn set_vring_err(&mut self, queue_index: usize, fd: &EventFd) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use VringConfigData;

    #[test]
    fn test_vring_config_data() {
        let mut config = VringConfigData {
            queue_max_size: 0x1000,
            queue_size: 0x2000,
            flags: 0x0,
            desc_table_addr: 0x4000,
            used_ring_addr: 0x5000,
            avail_ring_addr: 0x6000,
            log_addr: None,
        };

        assert_eq!(config.is_log_addr_valid(), true);
        assert_eq!(config.get_log_addr(), 0);

        config.flags = 0x1;
        assert_eq!(config.is_log_addr_valid(), false);
        assert_eq!(config.get_log_addr(), 0);

        config.log_addr = Some(0x7000);
        assert_eq!(config.is_log_addr_valid(), true);
        assert_eq!(config.get_log_addr(), 0x7000);

        config.flags = 0x0;
        assert_eq!(config.is_log_addr_valid(), true);
        assert_eq!(config.get_log_addr(), 0);
    }
}
