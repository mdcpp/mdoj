use leptos::*;

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="w-full bottom-0 bg-background p-2 border-t-2 border-primary flex flex-col justify-center items-center">
            <div>
                <p class="text-text">
                    "⚡Power⚡ by " <a href="https://github.com/mdcpp/mdoj" class="text-primary">
                        MDOJ
                    </a>
                </p>
            </div>
        </footer>
    }
}
