flat_mod! { read, write, pipe, fetch }

#[cfg(web_sys_unstable_apis)]
#[cfg_attr(docsrs, doc(cfg(web_sys_unstable_apis)))]
pub mod builder;