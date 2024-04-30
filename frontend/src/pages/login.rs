use ::grpc::backend::Role;
use leptos::*;
use leptos_router::use_navigate;

use crate::{
    components::*,
    config::*,
    error::*,
    grpc::{self, token_set_client},
};

#[component]
pub fn Login() -> impl IntoView {
    let username = create_rw_signal("".to_owned());
    let password = create_rw_signal("".to_owned());

    let submit =
        create_action(move |(username, password): &(String, String)| {
            let username = username.clone();
            let password = password.clone();

            let navigate = use_navigate();
            let (_, set_token) = use_token();
            async move {
                let mut token_set = token_set_client::TokenSetClient::new(
                    grpc::new_client().await?,
                );
                let resp = token_set
                    .create(grpc::LoginRequest {
                        username,
                        password,
                        expiry: None,
                    })
                    .await?;
                let resp = resp.into_inner();
                set_token(Some(Token {
                    token: resp.token.signature,
                    role: Role::try_from(resp.role).map_err(|_| {
                        ErrorKind::ServerError(ServerErrorKind::InvalidValue)
                    })?,
                }));
                navigate("/", Default::default());
                Result::<_>::Ok(())
            }
        });

    let disabled = Signal::derive(move || {
        submit.pending()() || username().is_empty() || password().is_empty()
    });

    let error_msg = move || {
        submit.value()().and_then(|r| r.err()).map(|e| match e {
            ErrorKind::NotFound => {
                "Username or password is incorrect".to_owned()
            }
            e => e.to_string(),
        })
    };

    view! {
        <main class="grow flex items-center justify-center">
            <form
                class="flex flex-col flex-nowrap justify-center items-center rounded-xl bg-lighten shadow-2xl shadow-secondary"
                on:submit=move |e| {
                    e.prevent_default();
                    submit.dispatch((username(), password()));
                    password.set("".to_owned());
                }
            >

                <img src="https://placehold.co/200" alt="Logo" class="mt-8 mb-4"/>

                <div class="p-4 flex flex-col">
                    <label for="username" class="text-text pb-2">
                        Username
                    </label>
                    <TextInput id="username" value=username/>
                </div>
                <div class="p-4 flex flex-col">
                    <label for="password" class="text-text pb-2">
                        Password
                    </label>
                    <TextInput kind="password" id="password" value=password/>
                </div>
                <p class="w-full text-red text-center">{error_msg}</p>
                <div class="p-4 w-full">
                    <Button kind="submit" class="w-full" disabled>
                        Login
                    </Button>
                </div>
            </form>
        </main>
    }
}
