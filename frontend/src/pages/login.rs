use crate::{components::*, grpc};
use leptos::*;

#[component]
pub fn Login() -> impl IntoView {
    let (username,set_username)=create_signal("".to_owned());
    let (password,set_password)=create_signal("".to_owned());

    let submit = create_action(move |_: &()| {
        let username = username();
        let password = password();
        async move {
            logging::log!("Click!");
            let mut token_set = grpc::TokenSetClient::new(grpc::new_client());
            let resp = token_set
                .create(grpc::LoginRequest {
                    username,
                    password,
                    expiry: None,
                })
                .await
                .unwrap();
            logging::log!("Token: {}", resp.get_ref().token.signature);
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
                    <Button kind="submit" class="w-full">
                        Login
                    </Button>
                </div>
            </form>
        </div>
    }
}
