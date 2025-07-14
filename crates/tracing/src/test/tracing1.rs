// NOTE: our tests are sensitive to the line numbers in this code!
//       if you do anything that changes line numbering, you'll
//       have to edit the test

use tracing::{debug, error, info, instrument, trace, warn_span};

pub fn run() {
    #[instrument]
    fn funkabloid() {
        debug!("hello world!");
        error!("hello world, but in an evil manner");
    }

    info!("woa");
    {
        let _s = warn_span!("hello!", two_plus_two = 2 + 2).entered();
        trace!("yaaa");
        funkabloid();
    }
}
