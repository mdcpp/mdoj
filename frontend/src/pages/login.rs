use crate::{
    components::*,
    config::{login_info, LoginInfo},
    grpc,
};
use anyhow::Ok;
use leptos::*;

#[component]
pub fn Login() -> impl IntoView {
    let (username, set_username) = create_signal("".to_owned());
    let (password, set_password) = create_signal("".to_owned());
    let (login_info, set_login_info, _) = login_info();
    logging::log!("Login...");

    let submit = create_action(move |_: &()| {
        let username = username();
        let password = password();
        async move {
            let mut token_set = grpc::TokenSetClient::new(grpc::new_client().await?);
            let resp = token_set
                .create(grpc::LoginRequest {
                    username,
                    password,
                    expiry: None,
                })
                .await?;
            let resp = resp.into_inner();
            set_login_info(Some(LoginInfo {
                token: resp.token.signature,
                // Todo
                permission: 0,
                expiry: 0,
            }));
            Ok(())
        }
    });

    view! {
        <div class="h-full flex items-center justify-center">
            <form
                class="flex flex-col flex-nowrap justify-center items-center rounded-xl bg-lighten border-2 border-primary"
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
                    <TextInput id="username" get=username set=set_username/>
                </div>
                <div class="p-4 flex flex-col">
                    <label for="password" class="text-text pb-2">
                        Password
                    </label>
                    <TextInput kind="password" id="password" get=password set=set_password/>
                </div>
                <div class="p-4 w-full">
                    <Button
                        kind="submit"
                        class="w-full disabled:opacity-70"
                        disabled=submit.pending()
                    >
                        Login
                    </Button>
                </div>
            </form>
        </div>
    }
}
