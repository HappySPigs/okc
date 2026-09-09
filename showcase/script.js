"use strict";

document.documentElement.classList.add("js");

const slides = Array.from(document.querySelectorAll(".slide"));
const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");

const elements = {
  chapterName: document.querySelector("#chapterName"),
  currentTitle: document.querySelector("#currentTitle"),
  currentSlide: document.querySelector("#currentSlide"),
  totalSlides: document.querySelector("#totalSlides"),
  progressBar: document.querySelector("#progressBar"),
  railItems: document.querySelector("#railItems"),
  outlineList: document.querySelector("#outlineList"),
  previousButton: document.querySelector("#previousButton"),
  nextButton: document.querySelector("#nextButton"),
  restartButton: document.querySelector("#restartButton"),
  outlineButton: document.querySelector("#outlineButton"),
  outlinePanel: document.querySelector("#outlinePanel"),
  outlineClose: document.querySelector("#outlineClose"),
  fullscreenButton: document.querySelector("#fullscreenButton"),
  printButton: document.querySelector("#printButton"),
  imageDialog: document.querySelector("#imageDialog"),
  imageClose: document.querySelector("#imageClose"),
  imagePreview: document.querySelector("#imagePreview"),
  imageCaption: document.querySelector("#imageCaption"),
};

let activeIndex = 0;
let lastFocusedElement = null;

const formatSlideNumber = (value) => String(value).padStart(2, "0");

function buildNavigation() {
  elements.totalSlides.textContent = formatSlideNumber(slides.length);

  slides.forEach((slide, index) => {
    const title = slide.dataset.title || `슬라이드 ${index + 1}`;
    const chapter = slide.dataset.chapter || "";

    const railButton = document.createElement("button");
    railButton.type = "button";
    railButton.className = "rail-item";
    railButton.setAttribute("aria-label", `${index + 1}번 슬라이드: ${title}`);
    railButton.title = title;
    railButton.addEventListener("click", () => goToSlide(index));
    elements.railItems.append(railButton);

    const listItem = document.createElement("li");
    const outlineButton = document.createElement("button");
    outlineButton.type = "button";
    outlineButton.dataset.slideIndex = String(index);

    const number = document.createElement("b");
    number.textContent = formatSlideNumber(index + 1);

    const label = document.createElement("span");
    label.textContent = title;

    const chapterLabel = document.createElement("small");
    chapterLabel.textContent = chapter;
    label.append(chapterLabel);

    outlineButton.append(number, label);
    outlineButton.addEventListener("click", () => {
      closeOutline();
      goToSlide(index);
    });

    listItem.append(outlineButton);
    elements.outlineList.append(listItem);
  });
}

function setActiveSlide(index, options = {}) {
  const nextIndex = Math.max(0, Math.min(slides.length - 1, index));
  const slide = slides[nextIndex];

  activeIndex = nextIndex;
  slides.forEach((item, itemIndex) => item.classList.toggle("is-active", itemIndex === activeIndex));

  const railButtons = Array.from(elements.railItems.children);
  const outlineButtons = Array.from(elements.outlineList.querySelectorAll("button"));

  railButtons.forEach((button, itemIndex) => {
    const isActive = itemIndex === activeIndex;
    button.classList.toggle("is-active", isActive);
    if (isActive) {
      button.setAttribute("aria-current", "page");
    } else {
      button.removeAttribute("aria-current");
    }
  });

  outlineButtons.forEach((button, itemIndex) => {
    const isActive = itemIndex === activeIndex;
    button.classList.toggle("is-active", isActive);
    if (isActive) {
      button.setAttribute("aria-current", "page");
    } else {
      button.removeAttribute("aria-current");
    }
  });

  elements.chapterName.textContent = slide.dataset.chapter || "OKC";
  elements.currentTitle.textContent = slide.dataset.title || "";
  elements.currentSlide.textContent = formatSlideNumber(activeIndex + 1);
  elements.progressBar.style.width = `${((activeIndex + 1) / slides.length) * 100}%`;
  elements.previousButton.disabled = activeIndex === 0;
  elements.nextButton.disabled = activeIndex === slides.length - 1;

  if (!options.skipHistory) {
    const nextHash = `#${slide.id}`;
    if (window.location.hash !== nextHash) {
      window.history.replaceState(null, "", nextHash);
    }
  }
}

function goToSlide(index, options = {}) {
  const nextIndex = Math.max(0, Math.min(slides.length - 1, index));
  const behavior = options.instant || reducedMotion.matches ? "auto" : "smooth";

  setActiveSlide(nextIndex);
  slides[nextIndex].scrollIntoView({ behavior, block: "start" });
}

function getInitialSlideIndex() {
  const hash = window.location.hash.replace("#", "");
  const hashIndex = slides.findIndex((slide) => slide.id === hash);
  return hashIndex >= 0 ? hashIndex : 0;
}

function observeSlides() {
  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          const index = slides.indexOf(entry.target);
          if (index >= 0) setActiveSlide(index);
        }
      });
    },
    {
      root: null,
      rootMargin: "-46% 0px -46% 0px",
      threshold: 0,
    },
  );

  slides.forEach((slide) => observer.observe(slide));
}

function openOutline() {
  if (elements.imageDialog.classList.contains("is-open")) closeImage();
  lastFocusedElement = document.activeElement;
  elements.outlinePanel.classList.add("is-open");
  elements.outlinePanel.setAttribute("aria-hidden", "false");
  document.body.classList.add("is-modal-open");
  window.setTimeout(() => elements.outlineClose.focus(), 80);
}

function closeOutline({ restoreFocus = true } = {}) {
  if (!elements.outlinePanel.classList.contains("is-open")) return;
  elements.outlinePanel.classList.remove("is-open");
  elements.outlinePanel.setAttribute("aria-hidden", "true");
  document.body.classList.remove("is-modal-open");

  if (restoreFocus && lastFocusedElement instanceof HTMLElement) {
    lastFocusedElement.focus();
  }
}

function openImage(trigger) {
  const imageSource = trigger.dataset.image;
  const caption = trigger.dataset.caption || "OKC showcase image";
  if (!imageSource) return;

  if (elements.outlinePanel.classList.contains("is-open")) closeOutline({ restoreFocus: false });
  lastFocusedElement = trigger;
  elements.imagePreview.src = imageSource;
  elements.imagePreview.alt = caption;
  elements.imageCaption.textContent = caption;
  elements.imageDialog.classList.add("is-open");
  elements.imageDialog.setAttribute("aria-hidden", "false");
  document.body.classList.add("is-modal-open");
  window.setTimeout(() => elements.imageClose.focus(), 80);
}

function closeImage() {
  if (!elements.imageDialog.classList.contains("is-open")) return;
  elements.imageDialog.classList.remove("is-open");
  elements.imageDialog.setAttribute("aria-hidden", "true");
  document.body.classList.remove("is-modal-open");

  window.setTimeout(() => {
    elements.imagePreview.src = "";
  }, 240);

  if (lastFocusedElement instanceof HTMLElement) {
    lastFocusedElement.focus();
  }
}

function trapFocus(event, container) {
  if (event.key !== "Tab") return;

  const focusable = Array.from(
    container.querySelectorAll('button:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])'),
  ).filter((element) => element instanceof HTMLElement && element.offsetParent !== null);

  if (focusable.length === 0) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];

  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

async function toggleFullscreen() {
  try {
    if (!document.fullscreenElement) {
      await document.documentElement.requestFullscreen();
    } else {
      await document.exitFullscreen();
    }
  } catch {
    elements.fullscreenButton.hidden = true;
  }
}

function isTypingContext(target) {
  if (!(target instanceof HTMLElement)) return false;
  return target.matches("input, textarea, select, [contenteditable='true']");
}

function handleGlobalKeyboard(event) {
  if (event.key === "Escape") {
    if (elements.imageDialog.classList.contains("is-open")) {
      closeImage();
      return;
    }
    if (elements.outlinePanel.classList.contains("is-open")) {
      closeOutline();
      return;
    }
  }

  if (elements.outlinePanel.classList.contains("is-open")) {
    trapFocus(event, elements.outlinePanel);
    return;
  }

  if (elements.imageDialog.classList.contains("is-open")) {
    trapFocus(event, elements.imageDialog);
    return;
  }

  if (isTypingContext(event.target) || event.metaKey || event.ctrlKey || event.altKey) return;

  const isInteractiveTarget = event.target instanceof HTMLElement && event.target.closest("button, a");

  if (["ArrowRight", "ArrowDown", "PageDown"].includes(event.key) || (event.key === " " && !isInteractiveTarget)) {
    event.preventDefault();
    goToSlide(activeIndex + 1);
  } else if (["ArrowLeft", "ArrowUp", "PageUp"].includes(event.key)) {
    event.preventDefault();
    goToSlide(activeIndex - 1);
  } else if (event.key === "Home") {
    event.preventDefault();
    goToSlide(0);
  } else if (event.key === "End") {
    event.preventDefault();
    goToSlide(slides.length - 1);
  } else if (event.key.toLowerCase() === "m") {
    event.preventDefault();
    openOutline();
  } else if (event.key.toLowerCase() === "f") {
    event.preventDefault();
    toggleFullscreen();
  }
}

function registerEvents() {
  elements.previousButton.addEventListener("click", () => goToSlide(activeIndex - 1));
  elements.nextButton.addEventListener("click", () => goToSlide(activeIndex + 1));
  elements.restartButton.addEventListener("click", () => goToSlide(0));
  elements.outlineButton.addEventListener("click", openOutline);
  elements.outlineClose.addEventListener("click", () => closeOutline());
  elements.outlinePanel.querySelector("[data-close-dialog]").addEventListener("click", () => closeOutline());
  elements.printButton.addEventListener("click", () => window.print());
  elements.fullscreenButton.addEventListener("click", toggleFullscreen);
  elements.imageClose.addEventListener("click", closeImage);
  elements.imageDialog.querySelector("[data-close-image]").addEventListener("click", closeImage);

  document.querySelectorAll(".zoomable").forEach((trigger) => {
    trigger.addEventListener("click", () => openImage(trigger));
  });

  document.addEventListener("keydown", handleGlobalKeyboard);

  window.addEventListener("hashchange", () => {
    const hashIndex = getInitialSlideIndex();
    if (hashIndex !== activeIndex) goToSlide(hashIndex);
  });

  if (!("requestFullscreen" in document.documentElement)) {
    elements.fullscreenButton.hidden = true;
  }
}

function initialize() {
  buildNavigation();
  registerEvents();
  observeSlides();

  const initialIndex = getInitialSlideIndex();
  setActiveSlide(initialIndex, { skipHistory: initialIndex === 0 && !window.location.hash });

  if (initialIndex > 0) {
    goToSlide(initialIndex, { instant: true });
  }
}

initialize();
