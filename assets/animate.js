export function animate() {

    const nav = document.querySelector("nav")
    const nav_height = nav.clientHeight;
    const navbarItems = document.querySelectorAll("nav a.navbar-item");
    let sections = new Map();
    const nav_handler = (entries) => {
        entries.map((entry) => sections.set(entry.target.id, entry.isIntersecting ? entry.intersectionRatio : 0));

        // Set top-most visible section as active
        let active
        for (const [section, value] of sections.entries()) {
            if (value > 0) {
                active = section
                break
            }
        }
        document.querySelector('nav a.navbar-item.is-active')?.classList.remove('is-active')
        document.querySelector('nav a.navbar-item[href="#' + active + '"]')?.classList.add("is-active")
        if (active === undefined)
            nav.classList.remove('scroll')
        else
            nav.classList.add('scroll')
    }
    const nav_observer = new IntersectionObserver(nav_handler, {
        rootMargin: `-${nav_height}px 0px 0px 0px`,
        threshold: 0.01
    });
    navbarItems.forEach(ni => {
        if (ni.href == null) return;
        let target = ni.hash.replace("#", "");
        target = document.getElementById(target)

        // Click handler
        ni.addEventListener("click", e => {
            e.preventDefault()
            if (target === null) {
                window.scrollTo({ top: 0, behavior: "smooth" });
            }
            else {
                document.querySelector('nav a.navbar-item.is-active')?.classList.remove('is-active')
                ni.classList.toggle("is-active")
                target.scrollIntoView({behavior: "smooth"})
            }
        })

        if (target !== null) {
            sections.set(target.id, 0)
            nav_observer.observe(target)
        }
    })

    document.querySelector(".hero a[href='#rewards']").addEventListener("click", e => {
        e.preventDefault()
        document.getElementById("rewards").scrollIntoView({behavior: "smooth"})
    })

    // attach_collapsible
    const collapsible = document.querySelectorAll(".is-collapsible");
    collapsible.forEach(c => {
        document.querySelectorAll(`[data-action="collapse"][href="#${c.id}"], [data-action="collapse"][data-target="${c.id}"]`).forEach(trigger => {
            trigger.addEventListener("click", e => {
                e.preventDefault()
                trigger.classList.toggle("is-active");

                const expanded = c.classList.contains("is-active")
                if (expanded)
                    collapse(c);
                else
                    expand(c);
            })
        })

        function collapse(e){
            c.classList.remove("animate__fadeIn")
            c.classList.add("animate__animated", "animate__fadeOut")
            e.style.height = '0px'
            c.classList.remove("is-active")
        }

        function expand(e){
            c.classList.remove("animate__fadeOut")
            c.classList.add("animate__animated", "animate__fadeIn")
            e.style.height = 'unset'
            c.classList.add("is-active")
        }

        const active = c.classList.contains("is-active")
        if (!active) {
            c.style.height = '0px'
        }

    })

    // Timeline spacers
    document.querySelectorAll(".timeline-phase").forEach(p => {
        const content = p.querySelector(".is-collapsible")
        const spacer = p.querySelector('.timeline-item.is-spacer')
        spacer?.classList.add("animate__animated")
        p.querySelectorAll("header > a").forEach(a => {
            a.addEventListener("click", e => spacer?.classList.toggle("animate__fadeIn"))
        })

        if (content?.classList.contains("is-active"))
            spacer?.classList.add("animate__fadeIn")
    })

    // Team observer
    let team_sync = new Map();
    const team_animation = (entries) => {
        entries.map((entry) => {
            if (entry.time - team_sync.get(entry.target) < 1000) return;

            if (entry.isIntersecting && entry.intersectionRatio === 1)
                entry.target.classList.add('animate__pulse')
            else
                entry.target.classList.remove('animate__pulse')

            team_sync.set(entry.target, entry.time)
        });
    }
    const observer = new IntersectionObserver(team_animation, { threshold: 1, });
    document.querySelectorAll('#team img').forEach(t => observer.observe(t));
}