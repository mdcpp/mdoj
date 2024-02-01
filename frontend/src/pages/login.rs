use crate::{
    components::*,
    config::*,
    grpc::{self, token_set_client},
};
use ::grpc::backend::Role;
use anyhow::Ok;
use leptos::*;
use leptos_router::use_navigate;

#[component]
pub fn Login() -> impl IntoView {
    let username = create_rw_signal("".to_owned());
    let password = create_rw_signal("".to_owned());

    let submit = create_action(move |_: &()| {
        let username = username();
        let password = password();
        let navigate = use_navigate();
        let (_, set_token) = use_token();
        async move {
            let mut token_set = token_set_client::TokenSetClient::new(grpc::new_client().await?);
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
                role: Role::try_from(resp.role)?,
            }));
            navigate("/", Default::default());
            Ok(())
        }
    });

    let is_valid = Signal::derive(move || {
        submit.pending()() || username().is_empty() || password().is_empty()
    });

    view! {
        <div class="h-full flex items-center justify-center">
            <form
                class="flex flex-col flex-nowrap justify-center items-center rounded-xl bg-lighten shadow-2xl shadow-secondary"
                on:submit=move |e| {
                    e.prevent_default();
                    submit.dispatch(());
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
                <div class="p-4 w-full">
                    <Button kind="submit" class="w-full disabled:opacity-70 disabled:cursor-not-allowed" disabled=is_valid>
                        Login
                    </Button>
                </div>
            </form>
        </div>
    }
}
