use leptos::{
    IntoView, component,
    prelude::{CustomAttribute, IntoAny, ServerFnError},
    view,
};

use crate::api::return_types::KnownErrors;

#[component]
pub fn LoginRedirect<T>(res: Result<T, ServerFnError>) -> impl IntoView {
    if res
        .is_err_and(|e| KnownErrors::parse_error(e).is_some_and(|e| e == KnownErrors::NotLoggedIn))
    {
        view! { <meta http-equiv="refresh" content="0; url=/login" /> }.into_any()
    } else {
        view! { "" }.into_any()
    }
}
