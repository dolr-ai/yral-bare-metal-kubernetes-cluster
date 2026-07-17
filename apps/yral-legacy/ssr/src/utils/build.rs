use anyhow::Result;

mod build_common {
    use anyhow::Result;

    pub fn build_common() -> Result<()> {
        // ml_feed gRPC client removed — recommendation service is no longer used
        Ok(())
    }
}

#[cfg(feature = "ssr")]
mod build_ssr {
    use std::{env, path::PathBuf};

    use anyhow::Result;

    fn build_gprc_client() -> Result<()> {
        let warehouse_events_proto = "contracts/projects/warehouse_events/warehouse_events.proto";
        let off_chain_proto = "contracts/projects/off_chain/off_chain.proto";
        let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

        tonic_build::configure()
            .build_client(true)
            .build_server(false)
            .out_dir(out_dir)
            .compile_protos(&[warehouse_events_proto, off_chain_proto], &["contracts"])?;
        Ok(())
    }

    pub fn build_ssr() -> Result<()> {
        build_gprc_client()?;

        Ok(())
    }
}

fn main() -> Result<()> {
    #[cfg(feature = "ssr")]
    {
        build_ssr::build_ssr()?;
    }

    build_common::build_common()?;

    Ok(())
}
