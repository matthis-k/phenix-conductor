use phenix_kernel::Kernel;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut kernel = Kernel::kernel_only();
    kernel.activate_all()?;
    Ok(())
}
