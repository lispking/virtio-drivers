#[cfg(test)]
pub mod fake;

use crate::{Error, Result, PAGE_SIZE};
use core::{marker::PhantomData, ptr::NonNull};

/// A virtual memory address in the address space of the program.
pub type VirtAddr = usize;

/// A physical address as used for virtio.
pub type PhysAddr = usize;

/// A region of contiguous physical memory used for DMA.
#[derive(Debug)]
pub struct Dma<H: Hal> {
    paddr: usize,
    pages: usize,
    _phantom: PhantomData<H>,
}

impl<H: Hal> Dma<H> {
    pub fn new(pages: usize, direction: BufferDirection) -> Result<Self> {
        let paddr = H::dma_alloc(pages, direction);
        if paddr == 0 {
            return Err(Error::DmaError);
        }
        Ok(Self {
            paddr,
            pages,
            _phantom: PhantomData::default(),
        })
    }

    pub fn paddr(&self) -> usize {
        self.paddr
    }

    pub fn vaddr(&self) -> usize {
        H::phys_to_virt(self.paddr)
    }

    pub fn raw_slice(&self) -> NonNull<[u8]> {
        let raw_slice =
            core::ptr::slice_from_raw_parts_mut(self.vaddr() as *mut u8, self.pages * PAGE_SIZE);
        NonNull::new(raw_slice).unwrap()
    }
}

impl<H: Hal> Drop for Dma<H> {
    fn drop(&mut self) {
        let err = H::dma_dealloc(self.paddr, self.pages);
        assert_eq!(err, 0, "failed to deallocate DMA");
    }
}

/// The interface which a particular hardware implementation must implement.
pub trait Hal {
    /// Allocates the given number of contiguous physical pages of DMA memory for virtio use.
    fn dma_alloc(pages: usize, direction: BufferDirection) -> PhysAddr;
    /// Deallocates the given contiguous physical DMA memory pages.
    fn dma_dealloc(paddr: PhysAddr, pages: usize) -> i32;
    /// Converts a physical address used for virtio to a virtual address which the program can
    /// access.
    ///
    /// This is used both for DMA regions allocated by `dma_alloc`, and for MMIO addresses within
    /// BARs read from the device (for the PCI transport).
    fn phys_to_virt(paddr: PhysAddr) -> VirtAddr;
    /// Shares the given memory range with the device, and returns the physical address that the
    /// device can use to access it.
    ///
    /// This may involve mapping the buffer into an IOMMU, giving the host permission to access the
    /// memory, or copying it to a special region where it can be accessed.
    fn share(buffer: NonNull<[u8]>, direction: BufferDirection) -> PhysAddr;
    /// Unshares the given memory range from the device and (if necessary) copies it back to the
    /// original buffer.
    fn unshare(paddr: PhysAddr, buffer: NonNull<[u8]>, direction: BufferDirection);
}

/// The direction in which a buffer is passed.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BufferDirection {
    /// The buffer may be read or written by the driver, but only read by the device.
    DriverToDevice,
    /// The buffer may be read or written by the device, but only read by the driver.
    DeviceToDriver,
    /// The buffer may be read or written by both the device and the driver.
    Both,
}
