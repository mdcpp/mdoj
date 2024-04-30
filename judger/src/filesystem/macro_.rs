macro_rules! chain_poll {
    ($poll:expr) => {{
        let poll_ = $poll;
        if poll_.is_pending() {
            return Poll::Pending;
        }
        match poll_ {
            Poll::Ready(x) => x,
            Poll::Pending => unreachable!(),
        }
    }};
}
macro_rules! report_poll {
    ($ans:expr) => {{
        if let Err(err) = $ans {
            return Poll::Ready(Err(err));
        }
        match $ans {
            Ok(x) => x,
            Err(_) => unreachable!(),
        }
    }};
}

pub(crate) use {chain_poll, report_poll};
