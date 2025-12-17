use crate::api::return_types::*;
use leptos::prelude::*;

#[component]
pub fn ClientLogin() -> impl IntoView {
    use crate::api::main_api::{Login, get_user_id_from_session};
    use leptos::either::{Either, EitherOf3};

    let login = ServerAction::<Login>::new();
    let logged_in = Resource::new(|| (), |_| async { get_user_id_from_session().await }); // this throws an error if the database can't find an account associated with the session

    view! {
        <div class="flex min-h-full flex-col justify-center px-6 py-12 lg:px-8">
            <div class="sm:mx-auto sm:w-full sm:max-w-sm">
                <img
                    src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=600"
                    alt="Your Company"
                    class="mx-auto h-10 w-auto dark:hidden"
                />
                <img
                    src="https://tailwindcss.com/plus-assets/img/logos/mark.svg?color=indigo&shade=500"
                    alt="Your Company"
                    class="mx-auto h-10 w-auto not-dark:hidden"
                />
                <h2 class="mt-10 text-center text-2xl/9 font-bold tracking-tight text-gray-900 dark:text-white">
                    Sign in to your account
                </h2>
            </div>

            <div class="mt-10 sm:mx-auto sm:w-full sm:max-w-sm">
                <Suspense>
                    // redirect to the homepage if the user's session id is already associated with an account
                    {move || match logged_in.get() {
                        Some(Ok(_)) => {
                            EitherOf3::A(view! { <meta http-equiv="refresh" content="0; url=/" /> })
                        }
                        Some(Err(_)) => {
                            EitherOf3::B(
                                view! {
                                    <ActionForm action=login>
                                        <div class="space-y-6">
                                            <div>
                                                <label
                                                    for="username"
                                                    class="block text-sm/6 font-medium text-gray-900 dark:text-gray-100"
                                                >
                                                    Username
                                                </label>
                                                <div class="mt-2">
                                                    <input
                                                        id="username"
                                                        type="text"
                                                        name="username"
                                                        required
                                                        autocomplete="username"
                                                        class="block w-full rounded-md bg-white px-3 py-1.5 text-base text-gray-900 outline-1 -outline-offset-1 outline-gray-300 placeholder:text-gray-400 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-600 sm:text-sm/6 dark:bg-white/5 dark:text-white dark:outline-white/10 dark:placeholder:text-gray-500 dark:focus:outline-indigo-500"
                                                        value=move || match login.value().get() {
                                                            Some(Err(e)) => {
                                                                match KnownErrors::parse_error(e) {
                                                                    Some(KnownErrors::LoginFailed { username }) => username,
                                                                    _ => "".to_string(),
                                                                }
                                                            }
                                                            _ => "".to_string(),
                                                        }
                                                    />
                                                </div>
                                            </div>

                                            <div>
                                                <div class="flex items-center justify-between">
                                                    <label
                                                        for="password"
                                                        class="block text-sm/6 font-medium text-gray-900 dark:text-gray-100"
                                                    >
                                                        Password
                                                    </label>
                                                // <div class="text-sm">
                                                // <a
                                                // href="#"
                                                // class="font-semibold text-indigo-600 hover:text-indigo-500 dark:text-indigo-400 dark:hover:text-indigo-300"
                                                // >
                                                // Forgot password?
                                                // </a>
                                                // </div>
                                                </div>
                                                <div class="mt-2">
                                                    <input
                                                        id="password"
                                                        type="password"
                                                        name="password"
                                                        required
                                                        autocomplete="current-password"
                                                        class="block w-full rounded-md bg-white px-3 py-1.5 text-base text-gray-900 outline-1 -outline-offset-1 outline-gray-300 placeholder:text-gray-400 focus:outline-2 focus:-outline-offset-2 focus:outline-indigo-600 sm:text-sm/6 dark:bg-white/5 dark:text-white dark:outline-white/10 dark:placeholder:text-gray-500 dark:focus:outline-indigo-500"
                                                    />
                                                </div>
                                            </div>

                                            <div>
                                                <button
                                                    type="submit"
                                                    class="flex w-full justify-center rounded-md bg-indigo-600 px-3 py-1.5 text-sm/6 font-semibold text-white shadow-xs hover:bg-indigo-500 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600 dark:bg-indigo-500 dark:shadow-none dark:hover:bg-indigo-400 dark:focus-visible:outline-indigo-500"
                                                >
                                                    Sign in
                                                </button>
                                            </div>
                                        </div>
                                    </ActionForm>
                                    <p class="mt-10 text-center text-sm/6 text-gray-500 dark:text-gray-400">
                                        Need an account?
                                        <a
                                            href="/signup"
                                            class="font-semibold text-indigo-600 hover:text-indigo-500 dark:text-indigo-400 dark:hover:text-indigo-300"
                                        >
                                            Register
                                        </a>
                                    </p>
                                },
                            )
                        }
                        None => EitherOf3::C(view! { "" }),
                    }}
                </Suspense>
                {move || match login.value().get() {
                    Some(Err(e)) => Either::Left(view! { <p>{e.to_string()}</p> }),
                    _ => Either::Right(view! { "" }),
                }}
            </div>
        </div>
    }
}

#[component]
pub fn ClientSignUp() -> impl IntoView {
    use crate::api::main_api::CreateUser;
    use leptos::either::Either;
    let signup = ServerAction::<CreateUser>::new();

    view! {
        <div class="mx-auto flex min-w-full flex-col items-center px-4 py-4">
            <Suspense>
                <div class="mx-auto flex min-w-full flex-col items-center px-4 py-4">

                    <br />

                    <ActionForm action=signup>

                        <div class="mx-auto flex min-w-full flex-col items-center px-4 py-4">

                            <input
                                class="shadow appearance-none border rounded py-2 px-2 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                                type="text"
                                name="username"
                                placeholder="username"
                                required
                            />
                            <br />
                            <input
                                class="shadow appearance-none border rounded py-2 px-2 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                                type="password"
                                name="password"
                                placeholder="password"
                                required
                            />
                            <br />
                            <input
                                class="shadow appearance-none border rounded py-2 px-2 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                                type="password"
                                name="confirm_password"
                                placeholder="confirm password"
                                required
                            />
                            <br />
                            <button
                                class="mt-3 rounded bg-purple-900 px-10 py-2 font-bold text-white hover:bg-blue-400"
                                type="submit"
                            >
                                "Sign up"
                            </button>
                        </div>
                    </ActionForm>
                    <a
                        href="/login"
                        class="mt-3 rounded bg-purple-900 px-10 py-2 font-bold text-white hover:bg-blue-400"
                        type="submit"
                    >
                        "Have an account? Sign in"
                    </a>
                </div>

            </Suspense>

            {move || match signup.value().get() {
                Some(Err(e)) => {
                    Either::Left(
                        view! {
                            <div class="mx-auto flex min-w-full flex-col items-center px-4 py-4">
                                <p>{e.to_string()}</p>
                            </div>
                        },
                    )
                }
                _ => Either::Right(view! { "" }),
            }}
        </div>
    }
}
