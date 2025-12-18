use super::nav::TopBar;
use crate::event_sourcing::journal::Permissions;
use leptos::prelude::*;
use uuid::Uuid;

struct Journal {
    pub id: Uuid,
    pub name: String,
}

fn journals() -> Vec<Journal> {
    vec![
        Journal {
            id: Uuid::new_v4(),
            name: "Personal".to_string(),
        },
        Journal {
            id: Uuid::new_v4(),
            name: "Business".to_string(),
        },
    ]
}

#[component]
pub fn JournalList() -> impl IntoView {
    view! {
        <div class="flex min-h-full flex-col justify-center px-6 py-12 lg:px-8">
            <div class="sm:mx-auto sm:w-full max-w-xs sm:max-w-sm">
                <img src="logo.svg" alt="Monkesto" class="mx-auto h-36 w-auto" />
                {journals()
                    .into_iter()
                    .map(|journal| {
                        view! {
                            <a
                                href=format!("/journal/{}", journal.id)
                                class="block mt-6 border border-gray-300 rounded-xl p-6 text-gray-700 hover:bg-blue-50 hover:border-blue-400 transition-colors duration-200 text-center"
                            >
                                <span class="text-xl font-semibold">{journal.name}</span>
                            </a>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

#[component]
pub fn JournalInvites() -> impl IntoView {
    use crate::api::main_api::{
        InviteToJournal, RespondToJournalInvite, get_associated_journals, get_journal_invites,
        get_user_id_from_session,
    };
    use leptos::either::{Either, EitherOf4};

    let invite_action = ServerAction::<InviteToJournal>::new();
    let response_action = ServerAction::<RespondToJournalInvite>::new();

    let user_id_resource =
        Resource::new(|| (), |_| async move { get_user_id_from_session().await });

    let journals_resource = Resource::new(
        || (),
        move |_| async move { get_associated_journals(user_id_resource.await).await },
    );

    let invites_resource = Resource::new(
        || (),
        move |_| async move { get_journal_invites(user_id_resource.await).await },
    );

    view! {
        <Suspense>
            {move || Suspend::new(async move {
                let user_id = match user_id_resource.await {
                    Ok(s) => s,
                    Err(_) => {
                        return EitherOf4::A(
                            view! { <meta http-equiv="refresh" content="0; url=/login" /> },
                        );
                    }
                };
                let journals = match journals_resource.await {
                    Ok(s) => s,
                    Err(e) => {
                        return EitherOf4::B(

                            view! {
                                <p>"An error occured while fetching journals: "{e.to_string()}</p>
                            },
                        );
                    }
                };
                let invites = match invites_resource.await {
                    Ok(s) => s,
                    Err(e) => {
                        return EitherOf4::C(

                            view! {
                                <p>"An error occured while fetching invites: "{e.to_string()}</p>
                            },
                        );
                    }
                };
                EitherOf4::D(

                    view! {
                        <TopBar user_id=user_id journals=journals.clone() />

                        {if let Some(selected) = journals.selected {
                            Either::Left(
                                view! {
                                    <ActionForm action=invite_action>
                                        <input
                                            type="hidden"
                                            name="journal_id"
                                            value=selected.get_id().to_string()
                                        />

                                        <input
                                            type="hidden"
                                            name="own_id"
                                            value=user_id.to_string()
                                        />

                                        <input
                                            type="hidden"
                                            name="permissions"
                                            // TODO: Add a selector for permissions
                                            value=serde_json::to_string(&Permissions::all())
                                                .expect("serialization of permissions failed")
                                        />

                                        <input
                                            class="shadow appearance-none border rounded py-2 px-2 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                                            type="text"
                                            name="invitee_username"
                                            placeholder="johndoe"
                                            required
                                        />

                                        <button class="mt-3 rounded bg-purple-900 px-10 py-2 font-bold text-white hover:bg-blue-400">
                                            "Invite to "{selected.get_name()} "!"
                                        </button>

                                    </ActionForm>

                                    {if let Some(Err(e)) = invite_action.value().get() {
                                        Either::Left(
                                            view! {
                                                <p>
                                                    "An error occured while creating the invitation: "
                                                    {e.to_string()}
                                                </p>
                                            },
                                        )
                                    } else {
                                        Either::Right(view! { "" })
                                    }}
                                },
                            )
                        } else {
                            Either::Right(view! { "" })
                        }}

                        {invites
                            .into_iter()
                            .map(|invite| {
                                view! {
                                    <h2 class="block mb-2 font-xl">{invite.name}</h2>
                                    <div class="flex">
                                        <ActionForm action=response_action>

                                            <input
                                                type="hidden"
                                                name="user_id"
                                                value=user_id.to_string()
                                            />

                                            <input
                                                type="hidden"
                                                name="journal_id"
                                                value=invite.id.to_string()
                                            />

                                            <input
                                                type="hidden"
                                                name="accepted"
                                                value=serde_json::to_string(&true)
                                                    .expect("failed to serialize true")
                                            />

                                            <button
                                                type="submit"
                                                class="mt-3 rounded bg-purple-900 px-2 py-2 font-bold text-white hover:bg-blue-400"
                                            >
                                                "Accept"
                                            </button>

                                        </ActionForm>
                                        <ActionForm action=response_action>
                                            <input
                                                type="hidden"
                                                name="user_id"
                                                value=user_id.to_string()
                                            />

                                            <input
                                                type="hidden"
                                                name="journal_id"
                                                value=invite.id.to_string()
                                            />

                                            <input
                                                type="hidden"
                                                name="accepted"
                                                value=serde_json::to_string(&false)
                                                    .expect("failed to serialize true")
                                            />

                                            <button
                                                type="submit"
                                                class="mt-3 rounded bg-purple-900 px-2 py-2 font-bold text-white hover:bg-blue-400"
                                            >
                                                "Decline"
                                            </button>
                                        </ActionForm>
                                    </div>
                                }
                            })
                            .collect_view()}

                        {if let Some(Err(e)) = response_action.value().get() {
                            Either::Left(view! { <p>"An error occured: "{e.to_string()}</p> })
                        } else {
                            Either::Right(view! { "" })
                        }}
                    },
                )
            })}
        </Suspense>
    }
}
