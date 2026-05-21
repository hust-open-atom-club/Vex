use super::types::{Snippet, SnippetCategory};

/// Construct the built-in snippet library (42 entries).
///
/// Order is fixed: Memory → CPU → Machine → Storage → Network →
/// Display → Debug → Kernel, each section in the order documented
/// in P4-6.
pub fn builtin_snippets() -> Vec<Snippet> {
    vec![
        // --- Memory (5) ---
        Snippet {
            name: "512M memory".to_string(),
            args: vec!["-m".to_string(), "512M".to_string()],
            category: SnippetCategory::Memory,
            description: Some("Allocate 512 MB of guest memory".to_string()),
        },
        Snippet {
            name: "1G memory".to_string(),
            args: vec!["-m".to_string(), "1G".to_string()],
            category: SnippetCategory::Memory,
            description: Some("Allocate 1 GB of guest memory".to_string()),
        },
        Snippet {
            name: "2G memory".to_string(),
            args: vec!["-m".to_string(), "2G".to_string()],
            category: SnippetCategory::Memory,
            description: Some("Allocate 2 GB of guest memory".to_string()),
        },
        Snippet {
            name: "4G memory".to_string(),
            args: vec!["-m".to_string(), "4G".to_string()],
            category: SnippetCategory::Memory,
            description: Some("Allocate 4 GB of guest memory".to_string()),
        },
        Snippet {
            name: "8G memory".to_string(),
            args: vec!["-m".to_string(), "8G".to_string()],
            category: SnippetCategory::Memory,
            description: Some("Allocate 8 GB of guest memory".to_string()),
        },
        // --- CPU (6) ---
        Snippet {
            name: "1 CPU".to_string(),
            args: vec!["-smp".to_string(), "1".to_string()],
            category: SnippetCategory::Cpu,
            description: Some("Single CPU core".to_string()),
        },
        Snippet {
            name: "2 CPUs".to_string(),
            args: vec!["-smp".to_string(), "2".to_string()],
            category: SnippetCategory::Cpu,
            description: Some("2 CPU cores".to_string()),
        },
        Snippet {
            name: "4 CPUs".to_string(),
            args: vec!["-smp".to_string(), "4".to_string()],
            category: SnippetCategory::Cpu,
            description: Some("4 CPU cores".to_string()),
        },
        Snippet {
            name: "8 CPUs".to_string(),
            args: vec!["-smp".to_string(), "8".to_string()],
            category: SnippetCategory::Cpu,
            description: Some("8 CPU cores".to_string()),
        },
        Snippet {
            name: "Host CPU".to_string(),
            args: vec!["-cpu".to_string(), "host".to_string()],
            category: SnippetCategory::Cpu,
            description: Some("Expose host CPU features (KVM only)".to_string()),
        },
        Snippet {
            name: "Cortex-A72".to_string(),
            args: vec!["-cpu".to_string(), "cortex-a72".to_string()],
            category: SnippetCategory::Cpu,
            description: Some("ARM Cortex-A72 (Raspberry Pi 4)".to_string()),
        },
        // --- Machine (4) ---
        Snippet {
            name: "virt machine".to_string(),
            args: vec!["-M".to_string(), "virt".to_string()],
            category: SnippetCategory::Machine,
            description: Some("Generic ARM virtual machine".to_string()),
        },
        Snippet {
            name: "q35 machine".to_string(),
            args: vec!["-M".to_string(), "q35".to_string()],
            category: SnippetCategory::Machine,
            description: Some("Modern x86_64 machine (PCIe, AHCI)".to_string()),
        },
        Snippet {
            name: "raspi3b machine".to_string(),
            args: vec!["-M".to_string(), "raspi3b".to_string()],
            category: SnippetCategory::Machine,
            description: Some("Raspberry Pi 3B emulation".to_string()),
        },
        Snippet {
            name: "microvm machine".to_string(),
            args: vec!["-M".to_string(), "microvm".to_string()],
            category: SnippetCategory::Machine,
            description: Some("Minimal x86_64 for fast boot".to_string()),
        },
        // --- Storage (7) ---
        Snippet {
            name: "virtio drive".to_string(),
            args: vec!["-drive".to_string(), "file=disk.img,if=virtio".to_string()],
            category: SnippetCategory::Storage,
            description: Some("Attach disk image via virtio (high performance)".to_string()),
        },
        Snippet {
            name: "IDE hard disk".to_string(),
            args: vec!["-hda".to_string(), "disk.img".to_string()],
            category: SnippetCategory::Storage,
            description: Some("Legacy IDE primary disk".to_string()),
        },
        Snippet {
            name: "CDROM ISO".to_string(),
            args: vec!["-cdrom".to_string(), "image.iso".to_string()],
            category: SnippetCategory::Storage,
            description: Some("Attach ISO as CDROM".to_string()),
        },
        Snippet {
            name: "raw image".to_string(),
            args: vec!["-drive".to_string(), "file=disk.img,format=raw".to_string()],
            category: SnippetCategory::Storage,
            description: Some("Raw disk image".to_string()),
        },
        Snippet {
            name: "qcow2 image".to_string(),
            args: vec![
                "-drive".to_string(),
                "file=disk.qcow2,format=qcow2".to_string(),
            ],
            category: SnippetCategory::Storage,
            description: Some("QCOW2 disk image".to_string()),
        },
        Snippet {
            name: "read-only drive".to_string(),
            args: vec![
                "-drive".to_string(),
                "file=disk.img,readonly=on".to_string(),
            ],
            category: SnippetCategory::Storage,
            description: Some("Attach disk in read-only mode".to_string()),
        },
        Snippet {
            name: "MTD flash".to_string(),
            args: vec!["-drive".to_string(), "file=flash.bin,if=mtd".to_string()],
            category: SnippetCategory::Storage,
            description: Some("Attach MTD flash (firmware ROMs)".to_string()),
        },
        // --- Network (4) ---
        Snippet {
            name: "user mode network".to_string(),
            args: vec![
                "-netdev".to_string(),
                "user,id=net0".to_string(),
                "-device".to_string(),
                "virtio-net,netdev=net0".to_string(),
            ],
            category: SnippetCategory::Network,
            description: Some("NAT user-mode networking with virtio-net".to_string()),
        },
        Snippet {
            name: "tap network".to_string(),
            args: vec![
                "-netdev".to_string(),
                "tap,id=net0".to_string(),
                "-device".to_string(),
                "virtio-net,netdev=net0".to_string(),
            ],
            category: SnippetCategory::Network,
            description: Some("Bridged TAP networking with virtio-net".to_string()),
        },
        Snippet {
            name: "no network".to_string(),
            args: vec!["-net".to_string(), "none".to_string()],
            category: SnippetCategory::Network,
            description: Some("Disable all networking".to_string()),
        },
        Snippet {
            name: "port forward 2222→22".to_string(),
            args: vec![
                "-netdev".to_string(),
                "user,id=net0,hostfwd=tcp::2222-:22".to_string(),
                "-device".to_string(),
                "virtio-net,netdev=net0".to_string(),
            ],
            category: SnippetCategory::Network,
            description: Some("Forward host TCP 2222 to guest SSH".to_string()),
        },
        // --- Display (5) ---
        Snippet {
            name: "no graphics".to_string(),
            args: vec!["-nographic".to_string()],
            category: SnippetCategory::Display,
            description: Some("Disable graphics, use serial console".to_string()),
        },
        Snippet {
            name: "no display".to_string(),
            args: vec!["-display".to_string(), "none".to_string()],
            category: SnippetCategory::Display,
            description: Some("Run headless (no QEMU window)".to_string()),
        },
        Snippet {
            name: "serial stdio".to_string(),
            args: vec!["-serial".to_string(), "stdio".to_string()],
            category: SnippetCategory::Display,
            description: Some("Redirect serial port to terminal".to_string()),
        },
        Snippet {
            name: "GTK display".to_string(),
            args: vec!["-display".to_string(), "gtk".to_string()],
            category: SnippetCategory::Display,
            description: Some("Open a GTK window (Linux desktop)".to_string()),
        },
        Snippet {
            name: "VNC server :0".to_string(),
            args: vec!["-display".to_string(), "vnc=:0".to_string()],
            category: SnippetCategory::Display,
            description: Some("Serve display via VNC on port 5900".to_string()),
        },
        // --- Debug (5) ---
        Snippet {
            name: "enable gdbserver".to_string(),
            args: vec!["-s".to_string()],
            category: SnippetCategory::Debug,
            description: Some("Listen for GDB on TCP 1234".to_string()),
        },
        Snippet {
            name: "freeze on start".to_string(),
            args: vec!["-S".to_string()],
            category: SnippetCategory::Debug,
            description: Some("Pause CPU until told to start (use with gdb)".to_string()),
        },
        Snippet {
            name: "gdb + freeze".to_string(),
            args: vec!["-s".to_string(), "-S".to_string()],
            category: SnippetCategory::Debug,
            description: Some("Wait for GDB before running any code".to_string()),
        },
        Snippet {
            name: "QEMU monitor stdio".to_string(),
            args: vec!["-monitor".to_string(), "stdio".to_string()],
            category: SnippetCategory::Debug,
            description: Some("Attach QEMU monitor to terminal".to_string()),
        },
        Snippet {
            name: "log interrupts + resets".to_string(),
            args: vec!["-d".to_string(), "int,cpu_reset".to_string()],
            category: SnippetCategory::Debug,
            description: Some("Trace interrupts and CPU resets".to_string()),
        },
        // --- Kernel (6) ---
        Snippet {
            name: "linux kernel".to_string(),
            args: vec!["-kernel".to_string(), "vmlinuz".to_string()],
            category: SnippetCategory::Kernel,
            description: Some("Boot a Linux kernel image directly".to_string()),
        },
        Snippet {
            name: "initrd".to_string(),
            args: vec!["-initrd".to_string(), "initrd.img".to_string()],
            category: SnippetCategory::Kernel,
            description: Some("Provide an initial RAM disk".to_string()),
        },
        Snippet {
            name: "device tree".to_string(),
            args: vec!["-dtb".to_string(), "device.dtb".to_string()],
            category: SnippetCategory::Kernel,
            description: Some("Load a device tree blob".to_string()),
        },
        Snippet {
            name: "kernel cmdline".to_string(),
            args: vec!["-append".to_string(), "console=ttyS0".to_string()],
            category: SnippetCategory::Kernel,
            description: Some("Append kernel command line".to_string()),
        },
        Snippet {
            name: "ARM serial cmdline".to_string(),
            args: vec!["-append".to_string(), "console=ttyAMA0".to_string()],
            category: SnippetCategory::Kernel,
            description: Some("Console on ARM PL011 UART".to_string()),
        },
        Snippet {
            name: "early printk".to_string(),
            args: vec!["-append".to_string(), "earlyprintk=ttyS0".to_string()],
            category: SnippetCategory::Kernel,
            description: Some("Enable early kernel printk".to_string()),
        },
    ]
}
