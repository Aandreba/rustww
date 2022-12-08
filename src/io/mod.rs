flat_mod! { read, write, fetch }

#[cfg(web_sys_unstable_apis)]
#[cfg_attr(docsrs, doc(cfg(web_sys_unstable_apis)))]
pub mod builder;