<?php
/**
 * Default page template — full content-editable via the block editor.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }
get_header();

while ( have_posts() ) :
	the_post();
	?>
	<section class="page-banner">
		<div class="container">
			<h1><?php the_title(); ?></h1>
			<?php if ( has_excerpt() ) : ?><p class="lead"><?php echo esc_html( get_the_excerpt() ); ?></p><?php endif; ?>
		</div>
	</section>

	<section class="section">
		<div class="container container--narrow stack entry-content">
			<?php the_content(); ?>
		</div>
	</section>
	<?php
endwhile;

get_footer();
