use crate::api::{
    main_api::{self},
    return_types::KnownErrors,
};
use leptos::prelude::*;

#[component]
pub fn LoginRedirect() -> impl IntoView {
    let user_id_resource = Resource::new(
        move || (),
        |_| async move { main_api::get_user_id_from_session().await },
    );
    view! {
        // redirect to the login page if the user's session id isn't associated with a logged in account
        <Suspense>
            {Suspend::new(async move {
                if user_id_resource
                    .await
                    .is_err_and(|e| {
                        Some(KnownErrors::NotLoggedIn) == KnownErrors::parse_error(e)
                    })
                {
                    view! { <meta http-equiv="refresh" content="0; url=/login" /> }.into_any()
                } else {
                    view! { "" }.into_any()
                }
            })}
        </Suspense>
    }
}
