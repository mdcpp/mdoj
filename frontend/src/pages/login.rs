use crate::{
    components::*,
    config::*,
    grpc::{token_set_client::*, *},
};
use anyhow::Ok;
use leptos::*;
use leptos_use::*;

#[component]
pub fn Login() -> impl IntoView {
    let (username, set_username) = create_signal("".to_owned());
    let (password, set_password) = create_signal("".to_owned());
    let (_, set_login_info, _) = use_login_info();

    let submit = create_action(move |_: &()| {
        let username = username();
        let password = password();
        async move {
            let mut token_set = TokenSetClient::new(new_client().await?);
            let resp = token_set
                .create(LoginRequest {
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

    let (login_info, ..) = use_login_info();
    view! {
        <RedirectIf condition=is_some(login_info) path="/"/>

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
